use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;

pub type Db = Arc<DatabaseConnection>;

pub async fn create_db_connection(database_url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    Database::connect(database_url).await
}
