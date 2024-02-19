use actix_web::{
    get, post,
    web::{Data, Json, Path},
    App, HttpResponse, HttpServer, Responder,
};
use chrono::Utc;
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct TransactionHistory {
    valor: i32,
    tipo: String,
    descricao: String,
    realizada_em: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct TransactionHistoryBalance {
    total: i32,
    data_extrato: chrono::DateTime<chrono::Utc>,
    limite: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct GetTransactionHistoryResponse {
    saldo: TransactionHistoryBalance,
    ultimas_transacoes: Vec<TransactionHistory>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTransactionRequest {
    valor: i32,
    tipo: String,
    descricao: String,
}

#[derive(Debug, Serialize)]
struct CreateTransactionResponse {
    limite: i32,
    saldo: i32,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: &'static str,
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
        return HttpResponse::NotFound().json(ErrorResponse {
            message: "'client id' is invalid",
        });
    }

    if req.valor < 1 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            message: "'valor' is invalid",
        });
    }

    if req.tipo != "c" && req.tipo != "d" {
        return HttpResponse::BadRequest().json(ErrorResponse {
            message: "'tipo' is invalid",
        });
    }

    if req.descricao.len() < 1 || req.descricao.len() > 10 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            message: "'descricao' is invalid",
        });
    }

    let client = match sqlx::query_as::<_, Client>("SELECT * FROM client WHERE id = $1")
        .bind(id.into_inner() as i32)
        .fetch_one(&data.pool)
        .await
    {
        Ok(wallet) => wallet,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                message: "'client id' is invalid",
            });
        }
    };

    let new_balance = match req.tipo.as_str() {
        "d" => {
            let new_balance = client.saldo - req.valor;
            if new_balance < -client.limite {
                return HttpResponse::UnprocessableEntity().json(ErrorResponse {
                    message: "transaction limit for the client is insufficient",
                });
            }
            new_balance
        }
        "c" => {
            let new_balance = client.saldo + req.valor;
            new_balance
        }
        _ => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                message: "'tipo' is invalid",
            });
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

    HttpResponse::Ok().json(CreateTransactionResponse {
        limite: client.limite,
        saldo: new_balance,
    })
}

#[get("/clientes/{id}/extrato")]
async fn get_transaction_history(data: Data<ApplicationData>, id: Path<u32>) -> impl Responder {
    if !CLIENTS.contains_key(&id.to_owned()) {
        return HttpResponse::NotFound().json(ErrorResponse {
            message: "'client id' is invalid",
        });
    }

    let client = match sqlx::query_as::<_, Client>("SELECT * FROM client WHERE id = $1")
        .bind(id.into_inner() as i32)
        .fetch_one(&data.pool)
        .await
    {
        Ok(wallet) => wallet,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                message: "'client id' is invalid",
            });
        }
    };

    let last_transactions = match sqlx::query_as::<_, TransactionHistory>(
        r#"
            SELECT valor, tipo, descricao, created_at AS realizada_em 
            FROM transaction 
            WHERE client_id = $1
            ORDER BY realizada_em DESC
            LIMIT 10
        "#,
    )
    .bind(&client.id)
    .fetch_all(&data.pool)
    .await
    {
        Ok(history) => history,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                message: "'client id' is invalid",
            })
        }
    };

    HttpResponse::Ok().json(GetTransactionHistoryResponse {
        saldo: TransactionHistoryBalance {
            total: client.saldo,
            limite: client.limite,
            data_extrato: Utc::now(),
        },
        ultimas_transacoes: last_transactions,
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("env var 'DATABASE_URL' not set");

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
            .service(get_transaction_history)
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
