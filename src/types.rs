//! Types definition used for handling returned data when generic API is used.

use std::collections::{HashMap, BTreeMap};
use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use std::str::FromStr;
use crate::error::{ErrorKind, Result};

pub type Amount = BigDecimal;
pub type Price = BigDecimal;
pub type Volume = BigDecimal;

pub trait BigDecimalConv {
    fn as_f64(&self) -> Result<f64>;
    fn as_f32(&self) -> Result<f32>;
}

impl BigDecimalConv for BigDecimal {
    fn as_f64(&self) -> Result<f64> {
        self.to_f64().ok_or(ErrorKind::BigDecimalTooLarge(self.clone()).into())
    }

    fn as_f32(&self) -> Result<f32> {
        self.to_f32().ok_or(ErrorKind::BigDecimalTooLarge(self.clone()).into())
    }
}

pub type Balances = HashMap<Currency, Amount>;

use chrono::prelude::*;
use crate::exchange::Exchange;
use derive_more::Display;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Channel {
    LiveTrades,
    LiveOrders,
    LiveOrderBook,
    LiveDetailOrderBook,
    LiveFullOrderBook,
}

#[derive(Debug)]
pub struct Ticker {
    /// UNIX timestamp in ms (when the response was received)
    pub timestamp: i64,
    /// The Pair corresponding to the Ticker returned (maybe useful later for asynchronous APIs)
    pub pair: Pair,
    /// Last trade price found in the history
    pub last_trade_price: Price,
    /// Lowest ask price found in Orderbook
    pub lowest_ask: Price,
    /// Highest bid price found in Orderbook
    pub highest_bid: Price,
    // Bittrex does not support Volume for ticker so volume could be None
    /// Last 24 hours volume (quote-volume)
    pub volume: Option<Volume>,
}

#[derive(Debug, Clone)]
pub struct Orderbook {
    /// UNIX timestamp in ms (when the response was received)
    pub timestamp: i64,
    /// The Pair corresponding to the Orderbook returned (maybe useful later for asynchronous APIs)
    pub pair: Pair,
    /// Vec containing the ask offers (by ascending price)
    pub asks: Vec<(Price, Volume)>,
    /// Vec containing the bid offers (by descending price)
    pub bids: Vec<(Price, Volume)>,
}

impl Orderbook {
    /// Convenient function that returns the average price from the orderbook
    /// Return None if Orderbook is empty
    /// `Average price = (lowest ask + highest bid)/2`
    pub fn avg_price(&self) -> Option<Price> {
        if self.asks.is_empty() || self.bids.is_empty() {
            return None;
        }
        Some(
            (self.asks[0].0.clone() + self.bids[0].0.clone())
                /
                BigDecimal::from_str("2.0").unwrap()
        )
    }
}

#[derive(Debug)]
pub struct LiveAggregatedOrderBook {
    pub depth: i8,
    pub pair: Pair,
    pub asks_by_price: BTreeMap<Price, (Price, Volume)>,
    pub bids_by_price: BTreeMap<Price, (Price, Volume)>,
    pub last_asks: Vec<(Price, Volume)>,
    pub last_bids: Vec<(Price, Volume)>,
}

const DEFAULT_BOOK_DEPTH: i8 = 5;

impl LiveAggregatedOrderBook {
    pub fn default(pair: Pair) -> LiveAggregatedOrderBook {
        LiveAggregatedOrderBook {
            depth: DEFAULT_BOOK_DEPTH,
            pair,
            asks_by_price: BTreeMap::new(),
            bids_by_price: BTreeMap::new(),
            last_asks: vec![],
            last_bids: vec![],
        }
    }

    pub fn order_book(&self) -> Orderbook {
        let asks: Vec<(Price, Volume)> = self.asks_by_price.iter().map(|(_, v)| v.clone()).take(self.depth as usize).rev().collect();
        let bids: Vec<(Price, Volume)> = self.bids_by_price.iter().rev().map(|(_, v)| v.clone()).take(self.depth as usize).rev().collect();
        Orderbook {
            timestamp: Utc::now().timestamp_millis(),
            pair: self.pair,
            asks,
            bids,
        }
    }

    pub fn latest_order_book(&mut self) -> Option<Orderbook> {
        let latest_order_book: Orderbook = self.order_book();
        if latest_order_book.asks == self.last_asks && latest_order_book.bids == self.last_bids {
            trace!("Order book top unchanged, not flushing");
            return None;
        } else {
            self.last_asks = latest_order_book.asks.clone();
            self.last_bids = latest_order_book.bids.clone();
            Some(latest_order_book)
        }
    }

    pub fn reset_asks<I>(&mut self, iter: I)
        where
            I: Iterator<Item=(BigDecimal, BigDecimal)> {
        for kp in iter {
            self.asks_by_price.entry(kp.0.clone()).or_insert(kp);
        }
    }

    pub fn reset_bids<I>(&mut self, iter: I)
        where
            I: Iterator<Item=(BigDecimal, BigDecimal)> {
        for kp in iter {
            self.bids_by_price.entry(kp.0.clone()).or_insert(kp);
        }
    }

    pub fn update_asks<I>(&mut self, iter: I)
        where
            I: Iterator<Item=(BigDecimal, BigDecimal)> {
        for kp in iter {
            self.update_ask(kp)
        }
    }

    pub fn update_ask(&mut self, kp: (BigDecimal, BigDecimal)) {
        let bids = &mut self.asks_by_price;
        if kp.1.is_zero() {
            bids.remove(&kp.0.clone());
        } else {
            bids.entry(kp.0.clone()).or_insert(kp);
        }
    }

    pub fn update_bids<I>(&mut self, iter: I)
        where
            I: Iterator<Item=(BigDecimal, BigDecimal)> {
        for kp in iter {
            self.update_ask(kp)
        }
    }

    pub fn update_bid(&mut self, kp: (BigDecimal, BigDecimal)) {
        let bids = &mut self.bids_by_price;
        if kp.1.is_zero() {
            bids.remove(&kp.0.clone());
        } else {
            bids.entry(kp.0.clone()).or_insert(kp);
        }
    }
}

#[derive(Debug)]
pub struct OrderInfo {
    /// UNIX timestamp in ms (when the response was received)
    pub timestamp: i64,
    /// This identifiers list is specific to the platform you use. You must store it somewhere if
    /// you want to modify/cancel the order later
    pub identifier: Vec<String>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum OrderType {
    BuyLimit,
    SellLimit,
    BuyMarket,
    SellMarket,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TradeType {
    Sell,
    Buy,
    None,
}

impl From<String> for TradeType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "sell" => TradeType::Sell,
            "buy" => TradeType::Buy,
            _ => TradeType::None,
        }
    }
}

impl From<i64> for TradeType {
    fn from(v: i64) -> Self {
        if v == 0 {
            return TradeType::Buy;
        }
        return TradeType::Sell;
    }
}

impl Into<i32> for TradeType {
    fn into(self) -> i32 {
        match self {
            TradeType::Buy => 0,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveTrade {
    /// UNIX timestamp in ms (when the event occured)
    pub event_ms: i64,
    /// The Pair corresponding to the Ticker returned (maybe useful later for asynchronous APIs)
    pub pair: String,
    /// Amount of the trade
    pub amount: f32,
    /// Price of the trade
    pub price: Price,
    /// Buy or Sell
    pub tt: TradeType,
}

#[derive(Debug, Clone)]
pub struct LiveOrder {
    /// UNIX timestamp in ms (when the event occured)
    pub event_ms: i64,
    /// The Pair corresponding to the Ticker returned (maybe useful later for asynchronous APIs)
    pub pair: String,
    /// Amount of the trade
    pub amount: f32,
    /// Price of the trade
    pub price: Price,
    /// Buy or Sell
    pub tt: TradeType,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub enum LiveEvent {
    LiveOrder(LiveOrder),
    LiveTrade(LiveTrade),
    LiveOrderbook(Orderbook),
    Noop,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct LiveEventEnveloppe(pub Exchange, pub LiveEvent);

/// Currency lists all currencies that can be traded on supported exchanges.
/// Update date : 27/10/2017.
/// Note : 1ST, 2GIVE, 8BIT have been renammed "_1ST", "_2GIVE" and "_8BIT" since variables name
/// cannot start with a number.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub enum Currency {
    _1ST,
    _2GIVE,
    _8BIT,
    ABY,
    ADA,
    ADC,
    ADT,
    ADX,
    AEON,
    AGRS,
    AM,
    AMP,
    AMS,
    ANT,
    APEX,
    APX,
    ARB,
    ARDR,
    ARK,
    AUR,
    BAT,
    BAY,
    BCC,
    BCH,
    BCN,
    BCY,
    BELA,
    BITB,
    BITCNY,
    BITS,
    BITZ,
    BLC,
    BLITZ,
    BLK,
    BLOCK,
    BAB,
    BNB,
    BNT,
    BOB,
    BRK,
    BRX,
    BSD,
    BSTY,
    BSV,
    BTA,
    BTC,
    BTCD,
    BTM,
    BTS,
    BURST,
    BYC,
    CAD,
    CANN,
    CCN,
    CFI,
    CLAM,
    CLOAK,
    CLUB,
    COVAL,
    CPC,
    CRB,
    CRBIT,
    CRW,
    CRYPT,
    CURE,
    CVC,
    DAR,
    DASH,
    DCR,
    DCT,
    DGB,
    DGC,
    DGD,
    DMD,
    DNT,
    DOGE,
    DOPE,
    DRACO,
    DTB,
    DTC,
    DYN,
    EBST,
    EDG,
    EFL,
    EGC,
    EMC,
    EMC2,
    ENRG,
    EOS,
    ERC,
    ETC,
    ETH,
    EUR,
    EXCL,
    EXP,
    FAIR,
    FC2,
    FCT,
    FLDC,
    FLO,
    FRK,
    FSC2,
    FTC,
    FUN,
    GAM,
    GAME,
    GBG,
    GBP,
    GBYTE,
    GCR,
    GEMZ,
    GEO,
    GHC,
    GLD,
    GNO,
    GNT,
    GOLOS,
    GP,
    GRC,
    GRS,
    GRT,
    GUP,
    HKG,
    HMQ,
    HUC,
    HYPER,
    HZ,
    ICN,
    INCNT,
    INFX,
    IOC,
    ION,
    IOP,
    J,
    JPY,
    KMD,
    KORE,
    KR,
    LBC,
    LGD,
    LMC,
    LSK,
    LTC,
    LUN,
    LXC,
    MAID,
    MANA,
    MAX,
    MCO,
    MEC,
    MEME,
    METAL,
    MLN,
    MND,
    MONA,
    MTL,
    MTR,
    MUE,
    MUSIC,
    MYST,
    MZC,
    NAUT,
    NAV,
    NBT,
    NEO,
    NEOS,
    NET,
    NEU,
    NLG,
    NMC,
    NMR,
    NOTE,
    NTRN,
    NXC,
    NXS,
    NXT,
    OC,
    OK,
    OMG,
    OMNI,
    ORB,
    PART,
    PASC,
    PAY,
    PDC,
    PINK,
    PIVX,
    PKB,
    POT,
    PPC,
    PRIME,
    PTC,
    PTOY,
    PXI,
    QRL,
    QTUM,
    QWARK,
    RADS,
    RBY,
    RDD,
    REP,
    RIC,
    RISE,
    RLC,
    ROOT,
    SAFEX,
    SALT,
    SBD,
    SC,
    SCOT,
    SCRT,
    SEQ,
    SFR,
    SHIFT,
    SIB,
    SJCX,
    SLG,
    SLING,
    SLR,
    SLS,
    SNGLS,
    SNRG,
    SNT,
    SOON,
    SPHR,
    SPR,
    SPRTS,
    SSD,
    START,
    STEEM,
    STEPS,
    STORJ,
    STR,
    STRAT,
    STV,
    SWIFT,
    SWING,
    SWT,
    SYNX,
    SYS,
    TES,
    THC,
    TIME,
    TIT,
    TIX,
    TKN,
    TKS,
    TRI,
    TRIG,
    TRK,
    TROLL,
    TRST,
    TRUST,
    TUSD,
    TX,
    U,
    UBQ,
    UFO,
    UNB,
    UNIQ,
    UNIT,
    UNO,
    USD,
    USDT,
    UTC,
    VIA,
    VIOR,
    VIRAL,
    VOX,
    VPN,
    VRC,
    VRM,
    VTC,
    VTR,
    WARP,
    WAVES,
    WINGS,
    XAUR,
    XBB,
    XBC,
    XC,
    XCO,
    XCP,
    XDG,
    XDN,
    XDQ,
    XEL,
    XEM,
    XLM,
    XMG,
    XMR,
    XMY,
    XPM,
    XPY,
    XQN,
    XRP,
    XSEED,
    XST,
    XTC,
    XTP,
    XVC,
    XVG,
    XWC,
    XZC,
    YBC,
    ZCL,
    ZEC,
    ZEN,
}

/// Pair lists all pairs that can be traded on supported exchanges.
/// Update date : 27/10/2017.
///
/// Order of quote currency <-> base currency is important. For example, Kraken supports ZEC_BTC
/// but Poloniex is doing the opposite inside their API: BTC_ZEC, which equal to 1/ZEC_BTC.
/// So: ZEC_BTC != BTC_ZEC but Poloniex ignores this and decided that BTC_ZEC = ZEC_BTC, so
/// that 1 ZEC = ZEC_BTC pair value. To standardize, the following pair uses the standard notation.
/// (so all Poloniex pair has been flipped)
///
/// Note : Kraken uses 'XBT' instead of 'BTC' (so the XBT/EUR pair becomes BTC/EUR).
///
/// To summarize, Kraken uses the pair 'ZEC_XBT', whereas Poloniex uses the 'BTC_ZEC' pair. With
/// the standardization proposed above these 2 pairs become 'ZEC_BTC', that are comparable in
/// value accross the 2 exchanges.
/// Pairs with "_d" at the end : dark pool
///
/// Note 2 : 1ST and 2GIVE have been renammed "_1ST" and "_2GIVE" since variables name cannot start
/// with a number.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, Display)]
#[allow(non_camel_case_types)]
pub enum Pair {
    _1ST_BTC,
    _1ST_ETH,
    _2GIVE_BTC,
    NONE,
    ABY_BTC,
    ADA_BTC,
    ADT_BTC,
    ADT_ETH,
    ADX_BTC,
    ADX_ETH,
    AEON_BTC,
    AGRS_BTC,
    AMP_BTC,
    ANT_BTC,
    ANT_ETH,
    APX_BTC,
    ARDR_BTC,
    ARK_BTC,
    AUR_BTC,
    BAT_BTC,
    BAT_ETH,
    BAY_BTC,
    BCC_BTC,
    BCC_ETH,
    BCC_USDT,
    BCH_BTC,
    BCH_ETH,
    BCH_EUR,
    BCH_USD,
    BCH_USDT,
    BCN_BTC,
    BCN_XMR,
    BCY_BTC,
    BELA_BTC,
    BITB_BTC,
    BLITZ_BTC,
    BLK_BTC,
    BLK_XMR,
    BLOCK_BTC,
    BAB_USD,
    BNB_USDT,
    BNT_BTC,
    BNT_ETH,
    BRK_BTC,
    BRX_BTC,
    BSD_BTC,
    BSV_BTC,
    BSV_USD,
    BTCD_BTC,
    BTCD_XMR,
    BTC_CAD,
    BTC_CAD_d,
    BTC_EUR,
    BTC_EUR_d,
    BTC_GBP,
    BTC_GBP_d,
    BTC_JPY,
    BTC_JPY_d,
    BTC_USD,
    BTC_USDT,
    BTC_USD_d,
    BTM_BTC,
    BTS_BTC,
    BTS_ETH,
    BURST_BTC,
    BYC_BTC,
    CANN_BTC,
    CFI_BTC,
    CFI_ETH,
    CLAM_BTC,
    CLOAK_BTC,
    CLUB_BTC,
    COVAL_BTC,
    CPC_BTC,
    CRB_BTC,
    CRB_ETH,
    CRW_BTC,
    CURE_BTC,
    CVC_BTC,
    CVC_ETH,
    DASH_BTC,
    DASH_ETH,
    DASH_EUR,
    DASH_USD,
    DASH_USDT,
    DASH_XMR,
    DCR_BTC,
    DCT_BTC,
    DGB_BTC,
    DGB_ETH,
    DGD_BTC,
    DGD_ETH,
    DMD_BTC,
    DNT_BTC,
    DNT_ETH,
    DOGE_BTC,
    DOPE_BTC,
    DTB_BTC,
    DYN_BTC,
    EBST_BTC,
    EDG_BTC,
    EFL_BTC,
    EGC_BTC,
    EMC2_BTC,
    EMC_BTC,
    ENRG_BTC,
    EOS_BTC,
    EOS_ETH,
    EOS_USD,
    EOS_USDT,
    ERC_BTC,
    ETC_BTC,
    ETC_ETH,
    ETC_EUR,
    ETC_USD,
    ETC_USDT,
    ETH_BTC,
    ETH_BTC_d,
    ETH_CAD,
    ETH_CAD_d,
    ETH_EUR,
    ETH_EUR_d,
    ETH_GBP,
    ETH_GBP_d,
    ETH_JPY,
    ETH_JPY_d,
    ETH_USD,
    ETH_USDT,
    ETH_USD_d,
    EUR_USD,
    EXCL_BTC,
    EXP_BTC,
    FAIR_BTC,
    FCT_BTC,
    FCT_ETH,
    FLDC_BTC,
    FLO_BTC,
    FTC_BTC,
    FUN_BTC,
    FUN_ETH,
    GAME_BTC,
    GAM_BTC,
    GAS_BTC,
    GAS_ETH,
    GBG_BTC,
    GBYTE_BTC,
    GCR_BTC,
    GEO_BTC,
    GLD_BTC,
    GNO_BTC,
    GNO_ETH,
    GNT_BTC,
    GNT_ETH,
    GOLOS_BTC,
    GRC_BTC,
    GRS_BTC,
    GUP_BTC,
    GUP_ETH,
    HMQ_BTC,
    HMQ_ETH,
    HUC_BTC,
    ICN_BTC,
    ICN_ETH,
    INCNT_BTC,
    INFX_BTC,
    IOC_BTC,
    ION_BTC,
    IOP_BTC,
    KMD_BTC,
    KORE_BTC,
    LBC_BTC,
    LGD_BTC,
    LGD_ETH,
    LMC_BTC,
    LSK_BTC,
    LSK_ETH,
    LTC_BTC,
    LTC_ETH,
    LTC_EUR,
    LTC_USD,
    LTC_USDT,
    LTC_XMR,
    LUN_BTC,
    LUN_ETH,
    MAID_BTC,
    MAID_XMR,
    MANA_BTC,
    MANA_ETH,
    MCO_BTC,
    MCO_ETH,
    MEME_BTC,
    MLN_BTC,
    MLN_ETH,
    MONA_BTC,
    MTL_BTC,
    MTL_ETH,
    MUE_BTC,
    MUSIC_BTC,
    MYST_BTC,
    MYST_ETH,
    NAUT_BTC,
    NAV_BTC,
    NBT_BTC,
    NEOS_BTC,
    NEO_BTC,
    NEO_ETH,
    NEO_USDT,
    NLG_BTC,
    NMC_BTC,
    NMR_BTC,
    NMR_ETH,
    NOTE_BTC,
    NXC_BTC,
    NXS_BTC,
    NXT_BTC,
    NXT_USDT,
    NXT_XMR,
    OK_BTC,
    OMG_BTC,
    OMG_ETH,
    OMG_USDT,
    OMNI_BTC,
    PART_BTC,
    PASC_BTC,
    PAY_BTC,
    PAY_ETH,
    PDC_BTC,
    PINK_BTC,
    PIVX_BTC,
    PKB_BTC,
    POT_BTC,
    PPC_BTC,
    PTC_BTC,
    PTOY_BTC,
    PTOY_ETH,
    QRL_BTC,
    QRL_ETH,
    QTUM_BTC,
    QTUM_ETH,
    QWARK_BTC,
    RADS_BTC,
    RBY_BTC,
    RDD_BTC,
    REP_BTC,
    REP_ETH,
    REP_EUR,
    REP_USDT,
    RIC_BTC,
    RISE_BTC,
    RLC_BTC,
    RLC_ETH,
    SAFEX_BTC,
    SALT_BTC,
    SALT_ETH,
    SBD_BTC,
    SC_BTC,
    SC_ETH,
    SEQ_BTC,
    SHIFT_BTC,
    SIB_BTC,
    SJCX_BTC,
    SLR_BTC,
    SLS_BTC,
    SNGLS_BTC,
    SNGLS_ETH,
    SNRG_BTC,
    SNT_BTC,
    SNT_ETH,
    SPHR_BTC,
    SPR_BTC,
    START_BTC,
    STEEM_BTC,
    STEEM_ETH,
    STORJ_BTC,
    STORJ_ETH,
    STRAT_BTC,
    STRAT_ETH,
    STR_BTC,
    STR_USDT,
    SWIFT_BTC,
    SWT_BTC,
    SYNX_BTC,
    SYS_BTC,
    THC_BTC,
    TIME_BTC,
    TIME_ETH,
    TIX_BTC,
    TIX_ETH,
    TKN_BTC,
    TKN_ETH,
    TKS_BTC,
    TRIG_BTC,
    TRST_BTC,
    TRST_ETH,
    TRUST_BTC,
    TUSD_BTC,
    TX_BTC,
    UBQ_BTC,
    UNB_BTC,
    USDT_USD,
    VIA_BTC,
    VOX_BTC,
    VRC_BTC,
    VRM_BTC,
    VTC_BTC,
    VTR_BTC,
    WAVES_BTC,
    WAVES_ETH,
    WINGS_BTC,
    WINGS_ETH,
    XAUR_BTC,
    XBC_BTC,
    XCP_BTC,
    XDG_BTC,
    XDN_BTC,
    XEL_BTC,
    XEM_BTC,
    XEM_ETH,
    XLM_BTC,
    XLM_ETH,
    XMG_BTC,
    XMR_BTC,
    XMR_ETH,
    XMR_EUR,
    XMR_USD,
    XMR_USDT,
    XMY_BTC,
    XPM_BTC,
    XRP_BTC,
    XRP_ETH,
    XRP_EUR,
    XRP_USD,
    XRP_USDT,
    XST_BTC,
    XVC_BTC,
    XVG_BTC,
    XWC_BTC,
    XTP_BTC,
    XZC_BTC,
    ZCL_BTC,
    ZEC_BTC,
    ZEC_ETH,
    ZEC_EUR,
    ZEC_USD,
    ZEC_USDT,
    ZEC_XMR,
    ZEN_BTC,
    ZRX_BTC,
    ZRX_ETH,
}
