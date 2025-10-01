use chrono::Utc;
use duckdb::{Connection, params};

use crate::fetch::Order;

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

pub fn get_latest_orders(persist_path: &str) -> anyhow::Result<Vec<Order>> {
    let conn = get_connection(persist_path)?;
    let mut statement = conn.prepare(
        r"SELECT
        type,
        blockchain,
        crypto_amount,
        crypto_symbol,
        fiat_amount,
        fiat_price,
        fiat_symbol
    FROM orders
    ORDER BY created_at DESC
    LIMIT 10;",
    )?;

    let orders = statement
        .query_map([], |row| {
            Ok(Order {
                ty: row.get(0)?,
                blockchain: row.get(1)?,
                crypto_amount: row.get(2)?,
                crypto_symbol: row.get(3)?,
                fiat_amount: row.get(4)?,
                fiat_price: row.get(5)?,
                fiat_symbol: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(orders)
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
            order.crypto_amount,
            order.crypto_symbol,
            order.fiat_amount,
            order.fiat_price,
            order.fiat_symbol,
        ],
    )?;

    Ok(())
}

pub fn get_connection(persist_path: &str) -> anyhow::Result<Connection> {
    let connection = Connection::open(persist_path)?;

    Ok(connection)
}
