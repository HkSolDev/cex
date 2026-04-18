use actix_web::{
    App, Error, HttpRequest, HttpResponse, HttpServer, Responder, delete, get, post,
    rt::task::yield_now, web,
};
use db::{create_connection_pool, delete_order, get_orders, lock_funds, settle_trade};
use domain::{Order, OrderId, OrderStatus, OrderType, Price, Qty, Side, Symbol, Trade, UserId};
use dotenvy;
use engine::{
    MatchingEngine,
    orderbook::{self, OrderBook},
};
use market_data::CandleEngine;
use serde_json::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH}; //do not know why its use
use tokio::{
    net::unix::pipe::Sender,
    sync::{broadcast, mpsc},
};

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
    sender: web::Data<HashMap<Symbol, mpsc::Sender<Order>>>,
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

    // 1. Decide what to pull out of the wallet based on the side!
    let asset_to_lock = match order.side {
        domain::Side::Buy => "USD",
        domain::Side::Sell => "BTC",
    };

    let amount_to_lock = match order.side {
        domain::Side::Buy => cost,         // Buyers lock total cash
        domain::Side::Sell => order.qty.0, // Sellers lock pure apples
    };

    //If the order symbol is not present in our market we will send a response to the user
    if !sender.contains_key(&order.symbol) {
        return HttpResponse::BadRequest().body("Invalid order the market is not present!");
    }

    // Here we got the tx the market for the specific symbol like BTC/USD
    let sender = sender.get(&order.symbol).unwrap();

    // The Risk Check!
    match db::lock_funds(pool_ref, payload.user_id.0, asset_to_lock, amount_to_lock).await {
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

#[get("/ws/market")]
async fn ws_market_feed(
    ws_data: web::Data<broadcast::Sender<Trade>>, //access the data here in this endpoint
    req: HttpRequest,
    stream: web::Payload,
) -> actix_web::Result<HttpResponse, actix_web::Error> {
    //This will help to transfrom the noraml http session to WebSocket
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let _ = session
        .text("Welcome to the Market Feed!".to_string())
        .await; //the let _ tell the rust just keep going and ignore the error as in the return type i am 
    let trade_tx = ws_data.get_ref();
    let mut candle_rx = trade_tx.subscribe();

    actix_web::rt::spawn(async move {
        while let Ok(trade) = candle_rx.recv().await {
            let trade = serde_json::to_string(&trade).unwrap();
            if session.text(trade).await.is_err() {
                break; // If the push fails (browser closed), kill this background task!
            }
        }
    });

    Ok(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = create_connection_pool(&url)
        .await
        .expect("Failed to create connection pool");

    //Create the broadcast channel for trades - the "megaphone"!
    let (trade_tx, _) = broadcast::channel::<Trade>(1000);
    let markets = vec!["BTC/USD", "ETH/USD"];

    let mut sender_map: HashMap<Symbol, mpsc::Sender<Order>> = HashMap::new();

    //Multiple market are creating here
    for market in markets {
        //Order in go the the stream
        let (tx, mut rx) = mpsc::channel::<Order>(10000);

        let trade_tx_clone = trade_tx.clone();

        tokio::spawn(async move {
            // this is the background thread
            // todo! what is the background thread?
            let mut orderbook = OrderBook::new(trade_tx_clone);
            while let Some(order) = rx.recv().await {
                orderbook.match_order(order);
            }
        });
        sender_map.insert(Symbol::from(market.to_string()), tx);
    }
    let incoming_order_data = web::Data::new(sender_map);
    let data = web::Data::new(db_pool.clone());

    let ws_data = web::Data::new(trade_tx.clone()); //I think we calone the broadcast channel here

    // 2. Each consumer subscribes
    let mut cashier_rx = trade_tx.clone().subscribe(); //why it need let mut cashier_rx = trade_tx.subscribe();

    // what this will do
    let candle_rx = trade_tx.clone().subscribe();

    // 3. Cashier
    tokio::spawn(async move {
        loop {
            match cashier_rx.recv().await {
                //here we receive the trade
                Ok(trade) => {
                    println!("the trade come in the candle_engine {:?}", trade);
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("Cashier lagged {n}");
                }
            }
        }
    });

    // 4. Candle Engine
    tokio::spawn(async move {
        let mut candle_engine = CandleEngine::new();
        candle_engine.run(candle_rx).await;
    });
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(ws_data.clone()) // pass the data
            .app_data(incoming_order_data.clone())
            .service(health_checker)
            .service(create_order)
            .service(get_orders_handler)
            .service(delete_order_id)
    })
    .bind(("127.0.0.1", 8080))? // Why bind in two ()
    .run()
    .await
}
