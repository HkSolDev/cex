use actix_web::{
    App, HttpResponse, HttpServer, Responder, delete, error::ErrorInternalServerError, get, post,
    rt::task::yield_now, web,
};
use db::{create_connection_pool, delete_order, get_orders, lock_funds};
use domain::{Order, OrderId, OrderStatus, OrderType, Price, Qty, Side, Symbol, UserId};
use dotenvy;
use engine::orderbook::OrderBook;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH}; //do not know why its use
use tokio::{net::unix::pipe::Sender, sync::mpsc};

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

#[get("/health_checker")]
async fn health_checker() -> impl Responder {
    format!("Hello All good!")
}

#[get("/orders")]
async fn get_orders_handler(pool: web::Data<PgPool>) -> HttpResponse {
    match db::get_orders(pool.get_ref()).await {
        Ok(orders) => HttpResponse::Ok().json(orders),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[post("/orders")]
async fn create_order(
    pool: web::Data<PgPool>,
    sender: web::Data<mpsc::Sender<Order>>,
    payload: web::Json<Order>,
) -> HttpResponse {
    println!("Order created: {:?}", payload);
    let order = Order {
        id: payload.id,
        user_id: payload.user_id,
        symbol: payload.symbol,
        side: payload.side,
        order_type: payload.order_type,
        price: payload.price,
        qty: payload.qty,
        filled_qty: payload.filled_qty,
        timestamp: now(),
        status: OrderStatus::Pending,
    };
    let cost = order.price.0 * order.qty.0;
    let pool_ref = pool.get_ref();
    let sender = sender.get_ref();
    // The Risk Check!
    match db::lock_funds(pool_ref, payload.user_id.0, "USD", cost).await {
        Ok(_) => {
            // Safe! Go ahead and create the order.
            match db::create_order(pool_ref, order.clone()).await {
                Ok(_) => {
                    if let Err(e) = sender.send(order.clone()).await {
                        println!("Failed to send to Engine: {}", e);
                    }
                    HttpResponse::Ok().json("Order created and funds locked!")
                }
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        Err(domain::DomainError::InsufficientFunds) => {
            // Bouncer says no!
            HttpResponse::BadRequest().body("Insufficient funds!")
        }
        Err(_) => {
            // Database crash during the check
            HttpResponse::InternalServerError().body("Risk check failed internally.")
        }
    }
}

#[delete("/order/{id}")]
async fn delete_order_id(
    pool: web::Data<PgPool>,
    path: web::Path<i64>,
) -> actix_web::Result<String> {
    let id = path.into_inner();
    let pool = pool.get_ref();
    match delete_order(pool, id).await {
        Ok(_) => Ok(format!("Order deleted: {}", id)),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = create_connection_pool(&url)
        .await
        .expect("Failed to create connection pool");
    let data = web::Data::new(db_pool);

    //Create the channel to store 10000 orders
    let (tx, mut rx) = mpsc::channel::<Order>(10000);

    tokio::spawn(async move {
        println!("Matching Engine Started!");

        let mut orderbook = OrderBook::new();

        while let Some(order) = rx.recv().await {
            // Process the order
            println!("Received order: {:?}", order);
            // Add your order matching logic here
            orderbook.match_order(order);
            // 3. Print the Best Bid/Ask so you can see the market moving!
            println!(
                "Current Market -> Bid: {:?} | Ask: {:?}",
                orderbook.best_bid_price(),
                orderbook.best_ask_price()
            );
        }
    });
    let sender_data = web::Data::new(tx);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(sender_data.clone()) // clone the sender data so each thread have axess to that
            .service(health_checker)
            .service(create_order)
            .service(get_orders_handler)
            .service(delete_order_id)
    })
    .bind(("127.0.0.1", 8080))? // Why bind in two ()
    .run()
    .await
}
