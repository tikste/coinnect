use crate::coinnect::Credentials;
use crate::exchange_bot::{ExchangeBot};
use crate::error::*;
use super::models::*;
use serde_json::Value;
use std::io::Read;
use actix::{Addr, Recipient};
use crate::types::{LiveEvent, Channel, Orderbook, Pair, LiveAggregatedOrderBook, LiveEventEnveloppe, LiveTrade};
use signalr_rs::hub::client::{HubClientError, HubClientHandler, HubClient, HubQuery, RestartPolicy, PendingQuery};
use serde::de::DeserializeOwned;
use libflate::deflate::Decoder;
use bigdecimal::BigDecimal;
use std::collections::{BTreeMap, HashMap, HashSet};
use crate::exchange::Exchange;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
pub struct BittrexStreamingApi {
    api_key: String,
    api_secret: String,
    customer_id: String,
    pub recipients: Vec<Recipient<LiveEventEnveloppe>>,
    books: Rc<RefCell<HashMap<Pair, LiveAggregatedOrderBook>>>,
    order_book_pairs: HashSet<Pair>,
    trade_pairs: HashSet<Pair>
}

pub struct BittrexBot {
    addr: Addr<HubClient>
}

impl ExchangeBot for BittrexBot {
    fn is_connected(&self) -> bool {
        unimplemented!()
    }
}

const BITTREX_HUB: &'static str = "c2";

impl BittrexStreamingApi {
    /// Create a new bittrex exchange bot, unavailable channels and currencies are ignored
    pub async fn new_bot<C: Credentials>(creds: Box<C>, channels: HashMap<Channel, HashSet<Pair>>, recipients: Vec<Recipient<LiveEventEnveloppe>>) -> Result<BittrexBot> {
        // Live order book pairs
        let mut map = channels.clone();
        let order_book_pairs: &HashSet<Pair> = map.entry(Channel::LiveFullOrderBook).or_default();
        // Live trade pairs
        let mut map = channels.clone();
        let trade_pairs : &HashSet<Pair> = map.entry(Channel::LiveTrades).or_default();

        let api = Box::new(BittrexStreamingApi {
            api_key: creds.get("api_key").unwrap_or_default(),
            api_secret: creds.get("api_secret").unwrap_or_default(),
            customer_id: creds.get("customer_id").unwrap_or_default(),
            recipients,
            books: Rc::new(RefCell::new(HashMap::new())),
            order_book_pairs: order_book_pairs.clone(),
            trade_pairs: trade_pairs.clone(),
        });
        let rc = api.books.clone();

        let mut books = rc.borrow_mut();
        for &pair in order_book_pairs {
            books.insert(pair, LiveAggregatedOrderBook::default(pair));
        }

        // SignalR Client
        let client = HubClient::new(BITTREX_HUB, "https://socket.bittrex.com/signalr/", 20, RestartPolicy::Always, api).await;
        match client {
            Ok(addr) => {
                if !order_book_pairs.is_empty() {
                    for &pair in order_book_pairs {
                        let currency = *super::utils::get_pair_string(&pair).unwrap();
                        addr.do_send(HubQuery::new(BITTREX_HUB.to_string(), "QueryExchangeState".to_string(), vec![currency.to_string()], "QE2".to_string()));
                    }
                }
                return Ok(BittrexBot { addr });
            }
            Err(e) => {
                return Err(ErrorKind::Hub(e).into());
            }
        }
    }

    fn deflate<T>(binary: &String) -> Result<T> where T: DeserializeOwned {
        let decoded = base64::decode(binary).map_err(|e| ErrorKind::Hub(HubClientError::Base64DecodeError(e)))?;
        let mut decoder = Decoder::new(&decoded[..]);
        let mut decoded_data: Vec<u8> = Vec::new();
        decoder.read_to_end(&mut decoded_data).map_err(|_| ErrorKind::Hub(HubClientError::InvalidData { data: vec!["cannot deflate".to_string()]}))?;
        let v: &[u8] = &decoded_data;
        serde_json::from_slice::<T>(v).map_err(|e| ErrorKind::Hub(HubClientError::ParseError(e)).into())
    }

    fn deflate_array<T>(a: &Value) -> Result<T> where T: DeserializeOwned {
        let data: Vec<String> = serde_json::from_value(a.clone())?;
        let binary = data.first().ok_or(ErrorKind::Hub(HubClientError::MissingData))?;
        BittrexStreamingApi::deflate::<T>(binary)
    }

    fn deflate_string<T>(a: &Value) -> Result<T> where T: DeserializeOwned {
        let binary: String = serde_json::from_value(a.clone())?;
        BittrexStreamingApi::deflate::<T>(&binary)
    }
}

impl HubClientHandler for BittrexStreamingApi {
    fn on_connect(&self) -> Vec<Box<PendingQuery>> {
        let mut conn_queries : Vec<Box<PendingQuery>> = vec![];
        if !self.trade_pairs.is_empty() || !self.order_book_pairs.is_empty() {
            let all_pairs : HashSet<Pair> = self.trade_pairs.union(&self.order_book_pairs).map(|&p| p).collect();
            let currencies : Vec<String> = all_pairs.iter().map(|p| (*super::utils::get_pair_string(p).unwrap()).to_string()).collect();
            info!("Bittrex : connecting to ExchangeDeltas for {:?}", &currencies);
            for currency in currencies {
                conn_queries.push(Box::new(HubQuery::new(BITTREX_HUB.to_string(), "SubscribeToExchangeDeltas".to_string(), vec![currency], "1".to_string())));
            }
        }
        conn_queries
    }

    fn error(&self, _: Option<&str>, _: &Value) {}

    fn handle(&mut self, method: &str, message: &Value) {
        let live_events = match method {
            "uE" => {
                let delta = BittrexStreamingApi::deflate_array::<MarketDelta>(message).unwrap();
                let pair = super::utils::get_pair_enum(delta.MarketName.as_str());
                if pair.is_none() {
                    return;
                }
                let mut events = vec![];
                let current_pair = *pair.unwrap();
                if self.order_book_pairs.contains(&current_pair) {
                    let mut books = self.books.borrow_mut();
                    let default_book = LiveAggregatedOrderBook::default(current_pair);
                    let mut agg = books.entry(current_pair).or_insert(default_book);
                    let asks = delta.Sells.into_iter().map(|op| (BigDecimal::from(op.Rate), BigDecimal::from(op.Quantity)));
                    agg.update_asks(asks);
                    let buys = delta.Buys.into_iter().map(|op| (BigDecimal::from(op.Rate), BigDecimal::from(op.Quantity)));
                    agg.update_bids(buys);
                    agg.latest_order_book().map(|ob| events.push(LiveEvent::LiveOrderbook(ob)));
                }
                if self.trade_pairs.contains(&current_pair) {
                    for fill in delta.Fills {
                        let lt = LiveTrade {
                            event_ms: fill.TimeStamp as i64,
                            pair: format!("{:?}", current_pair),
                            amount: fill.Quantity,
                            price: BigDecimal::from(fill.Rate),
                            tt: fill.OrderType.into(),
                        };
                        events.push(LiveEvent::LiveTrade(lt));
                    }
                }
                if events.is_empty() {
                    Err(())
                } else {
                    Ok(events)
                }
            }
            "uS" => {
                BittrexStreamingApi::deflate_array::<SummaryDeltaResponse>(message);
                Err(())
            }
            s if s.starts_with("QE") => {
                let state = BittrexStreamingApi::deflate_string::<ExchangeState>(message).unwrap();
                let pair = super::utils::get_pair_enum(state.MarketName.as_str());
                if pair.is_none() {
                    return;
                }
                let mut books = self.books.borrow_mut();
                let current_pair = *pair.unwrap();
                let default_book = LiveAggregatedOrderBook::default(current_pair);
                let mut agg = books.entry(current_pair).or_insert(default_book);
                let asks = state.Sells.into_iter().map(|op| (BigDecimal::from(op.R), BigDecimal::from(op.Q)));
                agg.reset_asks(asks);
                let bids = state.Buys.into_iter().map(|op| (BigDecimal::from(op.R), BigDecimal::from(op.Q)));
                agg.reset_bids(bids);
                let latest_order_book: Orderbook = agg.order_book();
                Ok(vec![LiveEvent::LiveOrderbook(latest_order_book.clone())])
            }
            _ => {
                trace!("Unknown message : method {:?} message {:?}", method, message);
                Err(())
            }
        };
        if let Ok(les) = live_events {
            let recipients = self.recipients.clone();
            for le in les {
                for r in &recipients {
                    let le: LiveEvent = le.clone();
                    r.do_send(LiveEventEnveloppe(Exchange::Bittrex, le)).unwrap();
                }
            }

        }
    }
}
