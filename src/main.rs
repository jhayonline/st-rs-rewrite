mod config;
mod controllers;
mod entities;
mod middleware;
mod models;
mod services;
mod utils;

use config::Config;
use controllers::auth::{login, me, signup};
use controllers::user::{change_password, get_profile, update_profile};
use middleware::auth::AuthMiddleware;
use middleware::cors::cors_handler;
use migration::{Migrator, MigratorTrait};
use salvo::prelude::*;
use std::sync::Arc;
use utils::logging::init_logging;

#[tokio::main]
async fn main() {
    init_logging();
    tracing::info!("Starting SlintTech Server...");

    let config = Config::from_env();
    tracing::info!("Configuration loaded successfully");

    let db_connection = config::database::create_db_connection(&config.database_url)
        .await
        .expect("Failed to connect to database");
    tracing::info!("Database connected successfully");

    tracing::info!("Running database migrations...");
    if let Err(e) = Migrator::up(&db_connection, None).await {
        tracing::error!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }
    tracing::info!("Migrations completed successfully");

    let db = Arc::new(db_connection);

    let router = Router::new()
        .hoop(cors_handler())
        .hoop(affix_state::inject(db.clone()))
        .hoop(affix_state::inject(config.clone()))
        .push(
            Router::with_path("api")
                // pub
                .push(
                    Router::with_path("auth")
                        .push(Router::with_path("login").post(login))
                        .push(Router::with_path("signup").post(signup)),
                )
                // proc
                .push(
                    Router::with_path("auth")
                        .push(Router::with_path("me").get(me).hoop(AuthMiddleware::new()))
                        .push(
                            Router::with_path("profile")
                                .get(get_profile)
                                .hoop(AuthMiddleware::new()),
                        )
                        .push(
                            Router::with_path("profile")
                                .put(update_profile)
                                .hoop(AuthMiddleware::new()),
                        )
                        .push(
                            Router::with_path("change-password")
                                .post(change_password)
                                .hoop(AuthMiddleware::new()),
                        ),
                )
                .push(Router::with_path("hello").get(hello)),
        );

    let acceptor = TcpListener::new(format!("{}:{}", config.server_host, config.server_port))
        .bind()
        .await;

    tracing::info!(
        "Server running on {}:{}",
        config.server_host,
        config.server_port
    );
    tracing::info!("API endpoints available:");
    tracing::info!("  POST /api/auth/login");
    tracing::info!("  POST /api/auth/signup");
    tracing::info!("  GET  /api/auth/me");
    tracing::info!("  GET  /api/auth/profile");
    tracing::info!("  PUT  /api/auth/profile");
    tracing::info!("  POST /api/auth/change-password");
    Server::new(acceptor).serve(router).await;
}

#[handler]
async fn hello() -> &'static str {
    "Hello from SlintTech API!"
}
