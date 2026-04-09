use actix_web::{
    App, HttpResponse, HttpServer, Responder, delete, error::ErrorInternalServerError, get, post, rt::task::yield_now, web
};
use db::{create_connection_pool, delete_order, get_orders};
use domain::{Order, OrderId, OrderStatus, OrderType, Price, Qty, Side, Symbol, UserId};
use dotenvy;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH}; //do not know why its use
//
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
async fn create_order(pool: web::Data<PgPool>, payload: web::Json<Order>) -> HttpResponse {
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

    match db::create_order(pool.get_ref(), order).await {
        //how to pass the orer in the order its
        //say it pass it as a createOrderTequest is this the good way or i just take the data form
        //teh paylod to pass it
        Ok(_) => HttpResponse::Ok().json("Order is creates"),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
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
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e))
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
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(health_checker)
            .service(create_order)
            .service(get_orders_handler)
            .service(delete_order_id)
    })
    .bind(("127.0.0.1", 8080))? // Why bind in two ()
    .run()
    .await
}
