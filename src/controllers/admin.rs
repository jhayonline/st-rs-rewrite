use crate::models::admin::{
    CreateUserRequest, DeleteUserRequest, GetUsersRequest, UpdateMentorAssignmentsRequest,
    UpdateUserRequest,
};
use crate::services::admin::AdminService;
use crate::utils::error::ApiError;
use salvo::prelude::*;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[handler]
pub async fn get_users(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let get_req: GetUsersRequest = req.parse_json().await?;

    // Use get_typed instead of get with string key
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    let response = AdminService::get_users(&db, &get_req).await?;
    Ok(Json(serde_json::to_value(response).unwrap()))
}

#[handler]
pub async fn create_user(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let create_req: CreateUserRequest = req.parse_json().await?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    let user = AdminService::create_user(&db, &create_req).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

#[handler]
pub async fn update_user(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let update_req: UpdateUserRequest = req.parse_json().await?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    let user = AdminService::update_user(&db, &update_req).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

#[handler]
pub async fn delete_user(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let delete_req: DeleteUserRequest = req.parse_json().await?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    AdminService::delete_user(&db, &delete_req.user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "User deleted successfully"
    })))
}

#[handler]
pub async fn get_stats(depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    let stats = AdminService::get_stats(&db).await?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

#[handler]
pub async fn get_mentors(depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    let mentors = AdminService::get_mentors(&db).await?;
    Ok(Json(serde_json::to_value(mentors).unwrap()))
}

#[handler]
pub async fn get_mentor_assignments(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = req.params();
    let mentee_id = query
        .get("menteeId")
        .ok_or_else(|| ApiError::bad_request("menteeId is required"))?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    let assignments = AdminService::get_mentor_assignments(&db, mentee_id).await?;
    Ok(Json(serde_json::to_value(assignments).unwrap()))
}

#[handler]
pub async fn update_mentor_assignments(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let update_req: UpdateMentorAssignmentsRequest = req.parse_json().await?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;

    AdminService::update_mentor_assignments(&db, &update_req.mentee_id, &update_req.assignments)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Mentor assignments updated successfully"
    })))
}
