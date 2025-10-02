use std::{collections::HashSet, time::Duration};

use clap::Parser;
use tokio::time::sleep;
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{
    EnvFilter, Layer, fmt::layer, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    args::Args,
    db::{get_latest_orders, init, insert_order},
    fetch::fetch,
};

mod args;
mod db;
mod fetch;

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
    let mut previous_orders = HashSet::from_iter(get_latest_orders(&args.persist_path)?);

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

        sleep(Duration::from_secs(args.fetch_interval)).await;
    }
}
