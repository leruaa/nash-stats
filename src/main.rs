use std::{collections::HashSet, error::Error, fmt::Display, time::Duration};

use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{
    EnvFilter, Layer, fmt::layer, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    args::Args,
    db::{init, insert_order},
};

mod args;
mod db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            layer().compact().with_target(false).with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
        )
        .init();

    let args = Args::parse();

    info!("Init DB");
    init(&args.persist_path)?;

    let client = reqwest::Client::new();
    let mut previous_orders = fetch(&client).await?;

    info!("Fetching orders...");
    loop {
        match fetch(&client).await {
            Ok(current_orders) => {
                let new_orders = current_orders
                    .difference(&previous_orders)
                    .collect::<Vec<_>>();

                if new_orders.len() == current_orders.len() {
                    warn!("New orders possibily missed");
                }

                for o in new_orders {
                    info!("New order: {o}");

                    if let Err(err) = insert_order(o, &args.persist_path) {
                        error!("Failed to insert order: {err}");
                    }
                }

                previous_orders = current_orders;
            }
            Err(err) => error!("{err}"),
        }

        sleep(Duration::from_millis(2000)).await;
    }
}

async fn fetch(client: &reqwest::Client) -> anyhow::Result<HashSet<Order>> {
    let current_orders: LatestOrders = client
        .get("https://app.nash.io/api/cash/latest_completed_orders")
        .send()
        .await?
        .json::<OrdersResponse>()
        .await?
        .try_into()?;

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
struct Order {
    #[serde(rename = "type")]
    ty: OrderType,
    blockchain: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    crypto_amount: f64,
    crypto_symbol: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    fiat_amount: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    fiat_price: f64,
    fiat_symbol: String,
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
enum OrderType {
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

fn from_str_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}
