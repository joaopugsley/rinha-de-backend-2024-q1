use actix_web::{web::Data, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;

pub struct ApplicationData {
    pool: Pool<Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("env var 'DATABASE_URL' not set");
    let port: u16 = env::var("PORT")
        .expect("env var 'PORT' not set")
        .parse()
        .expect("env var 'PORT' is not a number");

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("failed to connect to database pool");

    HttpServer::new(move || {
        App::new().app_data(Data::new(ApplicationData {
            pool: db_pool.clone(),
        }))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
