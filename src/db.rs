use chrono::Utc;
use duckdb::{Connection, params};

use crate::Order;

pub fn init(persist_path: &str) -> anyhow::Result<()> {
    let conn = get_connection(persist_path)?;

    conn.execute_batch(
        r"CREATE TABLE IF NOT EXISTS orders
            (
                created_at TIMESTAMP NOT NULL,
                type VARCHAR NOT NULL,
                blockchain VARCHAR NOT NULL,
                crypto_amount DOUBLE NOT NULL,
                crypto_symbol VARCHAR NOT NULL,
                fiat_amount DOUBLE NOT NULL,
                fiat_price DOUBLE NOT NULL,
                fiat_symbol VARCHAR NOT NULL,
            );",
    )?;

    Ok(())
}

pub fn insert_order(order: &Order, persist_path: &str) -> anyhow::Result<()> {
    let conn = get_connection(persist_path)?;
    let now = Utc::now();

    conn.execute(
        "INSERT INTO orders 
        (
            created_at,
            type,
            blockchain,
            crypto_amount,
            crypto_symbol,
            fiat_amount,
            fiat_price,
            fiat_symbol
        ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            now,
            order.ty.to_string(),
            order.blockchain,
            order.crypto_amount.parse::<f64>()?,
            order.crypto_symbol,
            order.fiat_amount.parse::<f64>()?,
            order.fiat_price.parse::<f64>()?,
            order.fiat_symbol,
        ],
    )?;

    Ok(())
}

pub fn get_connection(persist_path: &str) -> anyhow::Result<Connection> {
    let connection = Connection::open(persist_path)?;

    Ok(connection)
}
