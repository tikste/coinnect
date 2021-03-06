#[allow(non_snake_case)]

use serde::{Serialize, Deserialize};
use crate::types::LiveTrade;

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FillEntry {
    #[serde(alias = "F")]
    FillType: String,
    #[serde(alias = "I")]
    Id: i32,
    #[serde(alias = "OT")]
    OrderType: String,
    #[serde(alias = "P")]
    Price: f32,
    #[serde(alias = "Q")]
    Quantity: f32,
    #[serde(alias = "T")]
    TimeStamp: i64,
    #[serde(alias = "U")]
    Uuid: String,
    #[serde(alias = "t")]
    Total: f32,

}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OrderPair {
    #[serde(alias = "Q")]
    pub Q: f32,
    #[serde(alias = "R")]
    pub R: f32,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ExchangeState {
    #[serde(alias = "M")]
    pub MarketName: String,
    #[serde(alias = "N")]
    pub Nonce: i32,
    #[serde(alias = "Z")]
    pub Buys: Vec<OrderPair>,
    #[serde(alias = "S")]
    pub Sells: Vec<OrderPair>,
    #[serde(alias = "f")]
    Fills: Vec<FillEntry>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Order {
    #[serde(alias = "U")]
    Uuid: String,
    #[serde(alias = "OU")]
    OrderUuid: String,
    #[serde(alias = "I")]
    Id: i64,
    #[serde(alias = "E")]
    Exchange: String,
    #[serde(alias = "OT")]
    OrderType: String,
    #[serde(alias = "Q")]
    Quantity: f32,
    #[serde(alias = "q")]
    QuantityRemaining: f32,
    #[serde(alias = "X")]
    Limit: f32,
    #[serde(alias = "n")]
    CommissionPaid: f32,
    #[serde(alias = "P")]
    Price: f32,
    #[serde(alias = "PU")]
    PricePerUnit: f32,
    #[serde(alias = "Y")]
    Opened: i64,
    #[serde(alias = "C")]
    Closed: i64,
    #[serde(alias = "i")]
    IsOpen: bool,
    #[serde(alias = "CI")]
    CancelInitiated: bool,
    #[serde(alias = "K")]
    ImmediateOrCancel: bool,
    #[serde(alias = "k")]
    IsConditional: bool,
    #[serde(alias = "J")]
    Condition: String,
    #[serde(alias = "j")]
    ConditionTarget: f32,
    #[serde(alias = "u")]
    Updated: i64,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OrderDelta {
    #[serde(alias = "w")]
    AccountUuid: String,
    #[serde(alias = "N")]
    Nonce: i32,
    #[serde(alias = "TY")]
    Type: i32,
    #[serde(alias = "o")]
    Order: Order,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum TradeType {
    ADD = 0,
    REMOVE = 1,
    UPDATE = 2,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OrderLog {
    #[serde(alias = "TY")]
    pub Type: i32,
    #[serde(alias = "R")]
    pub Rate: f32,
    #[serde(alias = "Q")]
    pub Quantity: f32,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Fill {
    #[serde(alias = "FI")]
    FillId: i32,
    #[serde(alias = "OT")]
    pub OrderType: String,
    #[serde(alias = "R")]
    pub Rate: f32,
    #[serde(alias = "Q")]
    pub Quantity: f32,
    #[serde(alias = "T")]
    pub TimeStamp: u64,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct MarketDelta {
    #[serde(alias = "M")]
    pub MarketName: String,
    #[serde(alias = "N")]
    Nonce: i32,
    #[serde(alias = "Z")]
    pub Buys: Vec<OrderLog>,
    #[serde(alias = "S")]
    pub Sells: Vec<OrderLog>,
    #[serde(alias = "f")]
    pub Fills: Vec<Fill>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SummaryDelta {
    #[serde(alias = "M")]
    MarketName: String,
    #[serde(alias = "H")]
    High: f32,
    #[serde(alias = "L")]
    Low: f32,
    #[serde(alias = "V")]
    Volume: f32,
    #[serde(alias = "l")]
    Last: f32,
    #[serde(alias = "m")]
    BaseVolume: f32,
    #[serde(alias = "T")]
    TimeStamp: i64,
    #[serde(alias = "B")]
    Bid: f32,
    #[serde(alias = "A")]
    Ask: f32,
    #[serde(alias = "G")]
    OpenBuyOrders: i32,
    #[serde(alias = "g")]
    OpenSellOrders: i32,
    #[serde(alias = "PD")]
    PrevDay: f32,
    #[serde(alias = "x")]
    Created: i64,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SummaryDeltaResponse {
    #[serde(alias = "N")]
    Nonce: i32,
    #[serde(alias = "D")]
    Deltas: Vec<SummaryDelta>,
}

