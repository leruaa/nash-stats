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
    let args = Args::parse();

    info!("Start server...");
    init(&args.persist_path)?;

    tracing_subscriber::registry()
        .with(
            layer().compact().with_target(false).with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
        )
        .init();

    let client = reqwest::Client::new();
    let mut previous_orders = fetch(&client).await?;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[serde(rename = "type")]
    ty: OrderType,
    blockchain: String,
    crypto_amount: String,
    crypto_symbol: String,
    fiat_amount: String,
    fiat_price: String,
    fiat_symbol: String,
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} for {} {} at {} {}",
            self.ty,
            self.crypto_amount,
            self.crypto_symbol,
            self.fiat_amount,
            self.fiat_symbol,
            self.fiat_price,
            self.fiat_symbol
        )
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
