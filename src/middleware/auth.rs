use crate::config::Config;
use crate::entities::users;
use crate::utils::jwt::{extract_token, verify_token};
use salvo::http::StatusCode;
use salvo::prelude::*;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthUser {
    pub id: i32,
    pub pid: String,
    pub email: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "Admin"
    }

    pub fn is_mentor(&self) -> bool {
        self.role == "Mentor"
    }

    pub fn is_mentee(&self) -> bool {
        self.role == "Mentee"
    }
}

pub struct AuthMiddleware;

impl AuthMiddleware {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Handler for AuthMiddleware {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        // Get config from depot using get_typed (returns Result)
        let config = match depot.get_typed::<Config>() {
            Ok(c) => c,
            Err(_) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(serde_json::json!({
                    "error": "Configuration not found"
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Get database connection from depot
        let db = match depot.get_typed::<Arc<DatabaseConnection>>() {
            Ok(d) => d,
            Err(_) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(serde_json::json!({
                    "error": "Database connection not found"
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Extract token from Authorization header
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok());
        let token = match extract_token(auth_header) {
            Some(t) => t,
            None => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "error": "Unauthorized",
                    "details": "Missing or invalid Authorization header"
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Verify the token
        let claims = match verify_token(&token, &config.jwt_secret) {
            Ok(c) => c,
            Err(e) => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "error": "Unauthorized",
                    "details": format!("Invalid token: {}", e)
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Parse the UUID from the claims
        let user_pid = match Uuid::parse_str(&claims.sub) {
            Ok(uuid) => uuid,
            Err(e) => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "error": "Unauthorized",
                    "details": format!("Invalid user ID in token: {}", e)
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Get user from database using the parsed UUID
        let user = match users::Entity::find()
            .filter(users::Column::Pid.eq(user_pid))
            .one(db.as_ref())
            .await
        {
            Ok(Some(u)) => u,
            Ok(None) => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "error": "Unauthorized",
                    "details": "User not found"
                })));
                ctrl.skip_rest();
                return;
            }
            Err(e) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(serde_json::json!({
                    "error": "Database error",
                    "details": e.to_string()
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Create auth user and store in depot
        let auth_user = AuthUser {
            id: user.id,
            pid: user.pid.to_string(),
            email: user.email,
            role: user.role,
        };
        depot.insert("auth_user", auth_user);

        // Also store user ID for easy access
        depot.insert("user_id", user.id);

        // Continue to the next handler
        ctrl.call_next(req, depot, res).await;
    }
}

// Helper function to get authenticated user from depot
pub fn get_auth_user(depot: &Depot) -> Option<&AuthUser> {
    // Use get (not get_typed) when using string keys
    match depot.get::<AuthUser>("auth_user") {
        Ok(user) => Some(user),
        Err(_) => None,
    }
}

// Helper function to get user ID from depot
pub fn get_user_id(depot: &Depot) -> Option<i32> {
    match depot.get::<i32>("user_id") {
        Ok(id) => Some(*id),
        Err(_) => None,
    }
}

// Role-based authorization middleware
pub struct RequireRole {
    allowed_roles: Vec<String>,
}

impl RequireRole {
    pub fn new(roles: Vec<&str>) -> Self {
        Self {
            allowed_roles: roles.iter().map(|r| r.to_string()).collect(),
        }
    }
}

#[async_trait]
impl Handler for RequireRole {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        // Check if user is authenticated
        let auth_user = match get_auth_user(depot) {
            Some(u) => u,
            None => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "error": "Unauthorized",
                    "details": "Authentication required"
                })));
                ctrl.skip_rest();
                return;
            }
        };

        // Check if user has required role
        if !self.allowed_roles.contains(&auth_user.role) {
            res.status_code(StatusCode::FORBIDDEN);
            res.render(Json(serde_json::json!({
                "error": "Forbidden",
                "details": format!("Requires role: {:?}", self.allowed_roles)
            })));
            ctrl.skip_rest();
            return;
        }

        // Continue to the next handler
        ctrl.call_next(req, depot, res).await;
    }
}
