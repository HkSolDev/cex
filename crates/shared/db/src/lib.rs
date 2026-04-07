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
        let database_url = var("DATABASE_URL").expect("CRITICAL DATABASE_URL must be set in .env or system");
        let pool = create_connection_pool(&database_url).await.expect("Pool connection failed");
        let user_id = 1;
        let email = "[EMAIL_ADDRESS]";
        create_user(&pool, user_id, email).await.expect("Failed to create user");
        println!("User created successfully");
    }
}
