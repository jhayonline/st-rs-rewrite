use crate::config::Config;
use crate::models::auth::{LoginRequest, SignupRequest};
use crate::services::auth::AuthService;
use crate::utils::error::ApiError;
use salvo::prelude::*;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[handler]
pub async fn login(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let login_req: LoginRequest = req.parse_json().await?;

    let db = depot.get_typed::<Arc<DatabaseConnection>>().unwrap();
    let config = depot.get_typed::<Config>().unwrap();

    let response = AuthService::login(&db, &login_req, &config).await?;
    Ok(Json(serde_json::to_value(response).unwrap()))
}

#[handler]
pub async fn signup(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let signup_req: SignupRequest = req.parse_json().await?;

    let db = depot.get_typed::<Arc<DatabaseConnection>>().unwrap();
    let config = depot.get_typed::<Config>().unwrap();

    let user = AuthService::signup(&db, &signup_req, &config).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

#[handler]
pub async fn me(req: &mut Request, depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot.get_typed::<Arc<DatabaseConnection>>().unwrap();
    let user_id = req.extensions().get::<i32>().unwrap_or(&0);

    let user = AuthService::get_profile(&db, *user_id).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}
