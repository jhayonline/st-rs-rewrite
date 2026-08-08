mod config;
mod controllers;
mod middleware;
mod models;
mod services;
mod utils;

use config::Config;
use middleware::cors::cors_handler;
use salvo::logging::Logger;
use salvo::prelude::*;
use utils::logging::init_logging;

#[tokio::main]
async fn main() {
    init_logging();

    tracing::info!("Starting SlintTech Server...");

    let config = Config::from_env();
    tracing::info!("Configuration loaded successfully");

    let db = config::database::create_db_connection(&config.database_url)
        .await
        .expect("Failed to connect to database");
    tracing::info!("Database connected successfully");

    let router = Router::new()
        .hoop(Logger::new())
        .hoop(cors_handler())
        .get(hello);

    let acceptor = TcpListener::new(format!("{}:{}", config.server_host, config.server_port))
        .bind()
        .await;

    tracing::info!(
        "Server running on {}:{}",
        config.server_host,
        config.server_port
    );
    Server::new(acceptor).serve(router).await;
}

#[handler]
async fn hello() -> &'static str {
    "hello world"
}
