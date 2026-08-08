use sea_orm::Database;
use slinttech_server::config::Config;
use slinttech_server::migration::{Migrator, MigratorTrait};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();

    println!("Connecting to database...");
    let db = Database::connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    println!("Running migrations...");
    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    println!("Migrations completed successfully!");
}
