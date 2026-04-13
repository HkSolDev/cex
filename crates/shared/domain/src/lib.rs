use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
pub mod errors;
pub use errors::DomainError;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[sqlx(transparent)]
pub struct OrderId(pub i64);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[sqlx(transparent)]
pub struct UserId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Symbol(pub [u8; 8]); // E.g., "BTC-USD".

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[sqlx(transparent)]
pub struct Price(pub i64); // Integer cents

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[sqlx(transparent)]
pub struct Qty(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "side", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "order_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit,
    StopLimit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "order_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    PartialFilled,
    Filled,
    Cancelled,
}

#[derive(Debug,Clone)]
pub struct Trade{
    pub maker_user_id: i64,
    pub taker_user_id: i64,
    pub symbol: [u8;8],
    pub price: i64,
    pub qty: i64
}

impl From<i64> for OrderId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}
impl From<i64> for UserId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}
impl From<i64> for Price {
    fn from(v: i64) -> Self {
        Self(v)
    }
}
impl From<i64> for Qty {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

// For Symbol, which is a bit special:
impl From<Vec<u8>> for Symbol {
    fn from(v: Vec<u8>) -> Self {
        let mut bytes = [0u8; 8];
        let len = v.len().min(8);
        bytes[..len].copy_from_slice(&v[..len]);
        Self(bytes)
    }
}
impl From<String> for Symbol {
    fn from(s: String) -> Self {
        let mut bytes = [0u8; 8];
        let b = s.as_bytes();
        let len = b.len().min(8);
        bytes[..len].copy_from_slice(&b[..len]);
        Self(bytes)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: OrderId,
    pub user_id: UserId,
    pub symbol: Symbol,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Price,
    pub qty: Qty,
    pub filled_qty: Qty,
    pub timestamp: i64,
    pub status: OrderStatus,
}

impl Order {
    pub fn filled(&self) -> bool {
        self.qty == self.filled_qty
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppError {
    Internal(String),
    Validation(String),
    NotFound(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newtype_equality() {
        let order1 = OrderId(1);
        let order2 = OrderId(1);
        let order3 = OrderId(2);

        assert_eq!(order1, order2);
        assert_ne!(order1, order3);
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError::Validation("Invalid price".to_string());
        assert_eq!(format!("{}", err), "Validation error: Invalid price");
    }
}
