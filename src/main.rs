mod config;
mod controllers;
mod entities;
mod middleware;
mod models;
mod services;
mod utils;

use config::Config;
use controllers::admin::{
    create_user, delete_user, get_mentor_assignments, get_mentors, get_stats, get_users,
    update_mentor_assignments, update_user,
};
use controllers::auth::{login, me, signup};
use controllers::user::{change_password, get_profile, update_profile};
use middleware::auth::{AuthMiddleware, RequireRole};
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

    // Create the router
    let router = Router::new()
        .hoop(cors_handler())
        .hoop(affix_state::inject(db.clone()))
        .hoop(affix_state::inject(config.clone()))
        .push(
            Router::with_path("api")
                // public
                .push(
                    Router::with_path("auth")
                        .push(Router::with_path("login").post(login))
                        .push(Router::with_path("signup").post(signup)),
                )
                // protected
                .push(
                    Router::with_path("auth")
                        .hoop(AuthMiddleware::new())
                        .push(Router::with_path("me").get(me))
                        .push(Router::with_path("profile").get(get_profile))
                        .push(Router::with_path("profile").put(update_profile))
                        .push(Router::with_path("change-password").post(change_password)),
                )
                // admin
                .push(
                    Router::with_path("admin")
                        .hoop(AuthMiddleware::new())
                        .hoop(RequireRole::new(vec!["Admin"]))
                        .push(Router::with_path("users").post(get_users))
                        .push(Router::with_path("users/create").post(create_user))
                        .push(Router::with_path("users/update").put(update_user))
                        .push(Router::with_path("users/delete").delete(delete_user))
                        .push(Router::with_path("stats").get(get_stats))
                        .push(Router::with_path("mentors").get(get_mentors))
                        .push(Router::with_path("mentor-assignments").get(get_mentor_assignments))
                        .push(
                            Router::with_path("mentor-assignments/update")
                                .put(update_mentor_assignments),
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
    tracing::info!("  POST /api/admin/users");
    tracing::info!("  POST /api/admin/users/create");
    tracing::info!("  PUT  /api/admin/users/update");
    tracing::info!("  DELETE /api/admin/users/delete");
    tracing::info!("  GET  /api/admin/stats");
    tracing::info!("  GET  /api/admin/mentors");
    tracing::info!("  GET  /api/admin/mentor-assignments");
    tracing::info!("  PUT  /api/admin/mentor-assignments/update");
    Server::new(acceptor).serve(router).await;
}

#[handler]
async fn hello() -> &'static str {
    "Hello from SlintTech API!"
}
