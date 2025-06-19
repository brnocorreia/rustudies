use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use chrono::Utc;
use redis::JsonCommands;
use redis_macros::FromRedisValue;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    conn: Arc<Mutex<redis::Connection>>,
}

#[derive(Deserialize, Serialize, FromRedisValue, Debug)]
struct UserTokens {
    tokens: i32,
    last_updated: chrono::DateTime<Utc>,
}

async fn my_middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let mut conn = state.conn.lock().await;

    let authorization = &request.headers().get("Authorization");

    let token: &str = authorization
        .unwrap()
        .to_str()
        .unwrap()
        .split(" ")
        .collect::<Vec<&str>>()
        .get(1)
        .unwrap();

    let mut user_tokens = conn.json_get(token, "$").unwrap_or(UserTokens {
        tokens: 10,
        last_updated: chrono::Utc::now(),
    });

    dbg!(&user_tokens);

    let elapsed_hours: i64 = (chrono::Utc::now() - user_tokens.last_updated).num_hours();

    let updated_tokens = (user_tokens.tokens as i64 + elapsed_hours).min(10);

    if updated_tokens <= 0 {
        println!("No more tokens left: {token}");
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(Body::empty())
            .unwrap();
    }

    user_tokens.tokens = (updated_tokens - 1) as i32;
    user_tokens.last_updated = chrono::Utc::now();

    let _ = conn.json_set::<&str, &str, UserTokens, UserTokens>(token, "$", &user_tokens);

    let response = next.run(request).await;

    response
}

#[tokio::main]
async fn main() {
    let client = redis::Client::open("redis://localhost:6379/").unwrap();
    let con = client.get_connection().unwrap();

    let state = AppState {
        conn: Arc::new(Mutex::new(con)),
    };
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route_layer(middleware::from_fn_with_state(state.clone(), my_middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
