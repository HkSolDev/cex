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
