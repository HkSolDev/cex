use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("User {user_id} has insufficient funds: required {required}, but only has {actual}")]
    InSufficientFund {
        user_id: i64,
        required: i64,
        actual: i64,
    },
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("database error: {0}")]
    DatabaseError(sqlx::Error),
    #[error("failed to connect with the database")]
    DatabaseConnectionError,
    #[error("timeout occurred after {duration} seconds")]
    Timeout { duration: u64 },
}
