use domain::DomainError;
use domain::{Order, OrderId, Price, Symbol, UserId};
use sqlx::{PgPool, postgres::PgPoolOptions};
pub async fn create_connection_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

pub async fn create_user(pool: &PgPool, id: i64, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO users (id, email) VALUES ($1, $2)"#,
        id,
        email
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_order(pool: &PgPool, order: Order) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO orders (id, user_id, symbol, side, order_type, price, qty, filled_qty, timestamp, status) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        order.id as _,
        order.user_id as _,
      std::str::from_utf8(&order.symbol.0).unwrap_or("UNKNOWN").trim_matches(char::from(0)),
        order.side as domain::Side,
        order.order_type as domain::OrderType,
        order.price as Price,
        order.qty as _,
        order.filled_qty as _,
        order.timestamp as i64,
        order.status as domain::OrderStatus
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_order(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM orders WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_orders(pool: &PgPool) -> Result<Vec<Order>, sqlx::Error> {
    sqlx::query_as!(
        Order,
        r#"
        SELECT
          id, 
            user_id, 
            symbol, 
            side as "side: domain::Side", 
            order_type as "order_type: domain::OrderType", 
            price, 
            qty, 
            filled_qty, 
            timestamp, 
            status as "status: domain::OrderStatus" 
        FROM orders
        "#
    )
    .fetch_all(pool)
    .await
}

// Use your custom Error enum here instead of &str if possible!
pub async fn lock_funds(
    pool: &PgPool,
    user_id: i64,
    asset: &str,
    amount: i64,
) -> Result<(), DomainError> {
    // 1. Start the transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DomainError::DatabaseError(e))?;

    // 2. Execute the query
    let result = sqlx::query!(
        r#"
        UPDATE balances
        SET locked = locked + $3,
            free = free - $3  -- Use consistent column names
        WHERE user_id = $1 
          AND asset = $2 
          AND free >= $3
        "#,
        user_id,
        asset,
        amount
    )
    .execute(&mut *tx) // Use the transaction, not the pool!
    .await
    .map_err(|e| DomainError::DatabaseError(e))?;

    // 3. Check if any row was actually updated
    if result.rows_affected() == 0 {
        return Err(DomainError::InsufficientFunds);
    }
    // 4. Commit
    tx.commit()
        .await
        .map_err(|e| DomainError::DatabaseError(e))?;

    Ok(())
}

/// Called by the Matching Engine after a trade is confirmed.
/// Atomically transfers assets between the maker and taker using a single SQL transaction.
/// maker = the person whose order was already sitting in the book (e.g., Alice selling BTC)
/// taker = the person who just arrived and triggered the match (e.g., Bob buying BTC)
pub async fn settle_trade(
    pool: &PgPool,
    maker_user_id: i64, // Alice
    taker_user_id: i64, // Bob
    base_asset: &str,   // "BTC" — the thing being bought/sold
    quote_asset: &str,  // "USD" — the currency used to pay
    base_qty: i64,      // how much BTC was traded
    quote_qty: i64,     // how much USD was traded (price * qty)
) -> Result<(), DomainError> {
    // ATOMICITY: start the transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DomainError::DatabaseError(e))?;

    // 1. Alice gave away her BTC → subtract from her locked balance
    sqlx::query!(
        "UPDATE balances SET locked = locked - $1 WHERE user_id = $2 AND asset = $3",
        base_qty,
        maker_user_id,
        base_asset
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| DomainError::DatabaseError(e))?;

    // 2. Alice gets paid in USD → add to her free balance
    sqlx::query!(
        "UPDATE balances SET free = free + $1 WHERE user_id = $2 AND asset = $3",
        quote_qty,
        maker_user_id,
        quote_asset
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| DomainError::DatabaseError(e))?;

    // 3. Bob spent his USD → subtract from his locked balance
    sqlx::query!(
        "UPDATE balances SET locked = locked - $1 WHERE user_id = $2 AND asset = $3",
        quote_qty,
        taker_user_id,
        quote_asset
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| DomainError::DatabaseError(e))?;

    // 4. Bob gets his BTC → add to his free balance
    sqlx::query!(
        "UPDATE balances SET free = free + $1 WHERE user_id = $2 AND asset = $3",
        base_qty,
        taker_user_id,
        base_asset
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| DomainError::DatabaseError(e))?;

    // DURABILITY: all 4 queries passed — commit permanently to disk!
    tx.commit()
        .await
        .map_err(|e| DomainError::DatabaseError(e))?;

    Ok(())
}

#[cfg(test)]

mod tests {
    use super::*;
    use std::env::var;

    #[tokio::test]
    async fn test_database_connection() {
        dotenvy::dotenv().ok();
        let database_url =
            var("DATABASE_URL").expect("CRITICAL DATABASE_URL must be set in .env or system");
        let pool = create_connection_pool(&database_url)
            .await
            .expect("failed to connect to DB");

        let row: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .expect("Failed to execute Select 1");
        assert_eq!(row.0, 1);
        println!("Database connection successful");
    }

    #[tokio::test]
    async fn test_create_user() {
        dotenvy::dotenv().ok();
        let database_url =
            var("DATABASE_URL").expect("CRITICAL DATABASE_URL must be set in .env or system");
        let pool = create_connection_pool(&database_url)
            .await
            .expect("Pool connection failed");
        let user_id = 1;
        let email = "test@example.com";
        create_user(&pool, user_id, email)
            .await
            .expect("Failed to create user");
        println!("User created successfully");
    }
}
