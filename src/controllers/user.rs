use crate::middleware::auth::get_user_id;
use crate::models::user::{ChangePasswordRequest, UpdateProfileRequest};
use crate::services::user::UserService;
use crate::utils::error::ApiError;
use salvo::prelude::*;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[handler]
pub async fn update_profile(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let update_req: UpdateProfileRequest = req.parse_json().await?;

    let db = depot.get_typed::<Arc<DatabaseConnection>>().unwrap();
    let user_id = get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let user = UserService::update_profile(&db, user_id, &update_req).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

#[handler]
pub async fn change_password(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::value::Value>, ApiError> {
    let change_req: ChangePasswordRequest = req.parse_json().await?;

    let db = depot.get_typed::<Arc<DatabaseConnection>>().unwrap();
    let user_id = get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    UserService::change_password(&db, user_id, &change_req).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Password changed successfully"
    })))
}

#[handler]
pub async fn get_profile(depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot.get_typed::<Arc<DatabaseConnection>>().unwrap();
    let user_id = get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let user = UserService::get_profile(&db, user_id).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}
