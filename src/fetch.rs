use std::{collections::HashSet, error::Error, fmt::Display, hash::Hash, str::FromStr};

use anyhow::{anyhow, bail};
use approx::AbsDiffEq;
use duckdb::types::{FromSql, FromSqlError};
use serde::{Deserialize, Deserializer, Serialize};

pub async fn fetch(client: &reqwest::Client) -> anyhow::Result<HashSet<Order>> {
    let response_text = client
        .get("https://app.nash.io/api/cash/latest_completed_orders")
        .send()
        .await?
        .text()
        .await?;

    let current_orders = match serde_json::from_str::<OrdersResponse>(&response_text) {
        Ok(response) => LatestOrders::try_from(response)?,
        Err(_) => bail!("Failed to deserialize '{response_text}'"),
    };

    Ok(current_orders.into_set())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum OrdersResponse {
    Ok(LatestOrders),
    Err(OrdersError),
}

impl TryFrom<OrdersResponse> for LatestOrders {
    type Error = OrdersError;

    fn try_from(value: OrdersResponse) -> Result<Self, Self::Error> {
        match value {
            OrdersResponse::Ok(latest_orders) => Ok(latest_orders),
            OrdersResponse::Err(orders_error) => Err(orders_error),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestOrders {
    latest_orders: Vec<Order>,
}

impl LatestOrders {
    fn into_set(self) -> HashSet<Order> {
        HashSet::from_iter(self.latest_orders)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OrdersError {
    message: String,
}

impl Display for OrdersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for OrdersError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(rename = "type")]
    pub ty: OrderType,
    pub blockchain: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub crypto_amount: f64,
    pub crypto_symbol: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub fiat_amount: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub fiat_price: f64,
    pub fiat_symbol: String,
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
            && self.blockchain == other.blockchain
            && self
                .crypto_amount
                .abs_diff_eq(&other.crypto_amount, f64::EPSILON)
            && self.crypto_symbol == other.crypto_symbol
            && self
                .fiat_amount
                .abs_diff_eq(&other.fiat_amount, f64::EPSILON)
            && self.fiat_price.abs_diff_eq(&other.fiat_price, f64::EPSILON)
            && self.fiat_symbol == other.fiat_symbol
    }
}

impl Eq for Order {}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} for {} {} at {} {} on {}",
            self.ty,
            self.crypto_amount,
            self.crypto_symbol,
            self.fiat_amount,
            self.fiat_symbol,
            self.fiat_price,
            self.fiat_symbol,
            self.blockchain
        )
    }
}

impl Hash for Order {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.blockchain.hash(state);
        self.crypto_amount.to_bits().hash(state);
        self.crypto_symbol.hash(state);
        self.fiat_amount.to_bits().hash(state);
        self.fiat_price.to_bits().hash(state);
        self.fiat_symbol.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum OrderType {
    Buy,
    Sell,
}

impl Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Buy => write!(f, "buy"),
            OrderType::Sell => write!(f, "sell"),
        }
    }
}

impl FromStr for OrderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "buy" => Ok(OrderType::Buy),
            "sell" => Ok(OrderType::Sell),
            other => Err(anyhow!("Order type {other} not supported")),
        }
    }
}

impl FromSql for OrderType {
    fn column_result(value: duckdb::types::ValueRef<'_>) -> duckdb::types::FromSqlResult<Self> {
        value.as_str().and_then(|str| {
            str.parse::<OrderType>()
                .map_err(|err| FromSqlError::Other(err.into_boxed_dyn_error()))
        })
    }
}

fn from_str_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}
