use actix_web::{
    post,
    web::{Data, Json, Path},
    App, HttpResponse, HttpServer, Responder,
};
use dotenv::dotenv;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, FromRow, Pool, Postgres};
use std::{collections::HashMap, env};

struct ApplicationData {
    pool: Pool<Postgres>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Client {
    id: i32,
    limite: i32,
    saldo: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTransactionRequest {
    valor: i32,
    tipo: String,
    descricao: String,
}

struct CreateTransactionResponse {
    limite: i32,
    saldo: i32,
}

lazy_static! {
    static ref CLIENTS: HashMap<u32, u32> = [
        (1, 1000 * 100),
        (2, 800 * 100),
        (3, 10000 * 100),
        (4, 100000 * 100),
        (5, 5000 * 100),
    ]
    .iter()
    .cloned()
    .collect();
}

#[post("/clientes/{id}/transacoes")]
async fn create_transaction(
    data: Data<ApplicationData>,
    id: Path<u32>,
    req: Json<CreateTransactionRequest>,
) -> impl Responder {
    if !CLIENTS.contains_key(&id.to_owned()) {
        return HttpResponse::NotFound();
    }

    if req.valor < 1 {
        return HttpResponse::BadRequest();
    }

    if req.tipo != "c" && req.tipo != "d" {
        return HttpResponse::BadRequest();
    }

    if req.descricao.len() < 1 || req.descricao.len() > 10 {
        return HttpResponse::BadRequest();
    }

    let client = match sqlx::query_as::<_, Client>("SELECT * FROM client WHERE id = $1")
        .bind(id.into_inner() as i32)
        .fetch_one(&data.pool)
        .await
    {
        Ok(wallet) => wallet,
        Err(_) => {
            return HttpResponse::InternalServerError();
        }
    };

    let new_balance = match req.tipo.as_str() {
        "d" => {
            let new_balance = client.saldo - req.valor;
            if new_balance < -client.limite {
                return HttpResponse::UnprocessableEntity();
            }
            new_balance
        }
        "c" => {
            let new_balance = client.saldo + req.valor;
            new_balance
        }
        _ => {
            return HttpResponse::BadRequest();
        }
    };

    sqlx::query("UPDATE client SET saldo = $1 WHERE id = $2")
        .bind(&new_balance)
        .bind(&client.id)
        .execute(&data.pool)
        .await
        .expect("failed to updated client balance");

    sqlx::query(
        "INSERT INTO transaction (client_id, valor, tipo, descricao) VALUES ($1, $2, $3, $4)",
    )
    .bind(&client.id)
    .bind(&req.valor)
    .bind(&req.tipo)
    .bind(&req.descricao)
    .execute(&data.pool)
    .await
    .expect("failed to register transaction");

    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("env var 'DATABASE_URL' not set");
    let port: u16 = env::var("PORT")
        .expect("env var 'PORT' not set")
        .parse()
        .expect("env var 'PORT' is not a number");

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(ApplicationData {
                pool: db_pool.clone(),
            }))
            .service(create_transaction)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
