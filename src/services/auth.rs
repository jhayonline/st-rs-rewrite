use crate::config::Config;
use crate::entities::users;
use crate::models::auth::{AuthResponse, LoginRequest, SignupRequest, UserProfile};
use crate::models::user::UserResponse;
use crate::utils::error::ApiError;
use crate::utils::jwt::{Claims, generate_token};

use bcrypt::{DEFAULT_COST, hash, verify};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::sync::Arc;

pub struct AuthService;

impl AuthService {
    pub async fn login(
        db: &Arc<DatabaseConnection>,
        req: &LoginRequest,
        config: &Config,
    ) -> Result<AuthResponse, ApiError> {
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(req.email.clone()))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::unauthorized("Invalid email or password"))?;

        if !verify(&req.password, &user.password)? {
            return Err(ApiError::unauthorized("Invalid email or password"));
        }

        if user.status == "pending" {
            return Err(ApiError::forbidden("Your account is pending approval"));
        }
        if user.status == "rejected" {
            return Err(ApiError::forbidden("Your account has been rejected"));
        }
        if user.status == "suspended" {
            return Err(ApiError::forbidden("Your account has been suspended"));
        }

        let claims = Claims::new(user.pid, &user.email, &user.role, config.jwt_expiration);
        let token = generate_token(&claims, &config.jwt_secret)?;

        let profile = UserProfile {
            id: user.id,
            email: user.email,
            full_name: user.name,
            role: user.role,
            membership_category: user.membership_category,
            career_path: user.career_path,
            specialization: user.specialization,
            status: user.status,
            membership_enabled: user.membership_enabled,
            membership_paid: user.membership_paid,
            contract_file_url: user.contract_file_url,
            community_link: user.community_link,
            created_at: user.created_at,
        };

        Ok(AuthResponse { token, profile })
    }

    pub async fn signup(
        db: &Arc<DatabaseConnection>,
        req: &SignupRequest,
        _config: &Config,
    ) -> Result<UserResponse, ApiError> {
        let existing = users::Entity::find()
            .filter(users::Column::Email.eq(&req.email))
            .one(db.as_ref())
            .await?;

        if existing.is_some() {
            return Err(ApiError::conflict("Email already registered"));
        }

        let password_hash = hash(&req.password, DEFAULT_COST)?;
        let role = req.role.clone().unwrap_or_else(|| "Mentee".to_string());

        let user = users::ActiveModel {
            email: ActiveValue::Set(req.email.clone()),
            password: ActiveValue::Set(password_hash),
            name: ActiveValue::Set(req.full_name.clone()),
            role: ActiveValue::Set(role.clone()),
            membership_category: ActiveValue::Set(req.membership_category.clone()),
            career_path: ActiveValue::Set(req.career_path.clone()),
            specialization: ActiveValue::Set(req.specialization.clone()),
            status: ActiveValue::Set("pending".to_string()),
            membership_enabled: ActiveValue::Set(role == "Mentee" || role == "Mentor"),
            membership_paid: ActiveValue::Set(false),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await?;

        Ok(UserResponse::from(user))
    }

    pub async fn get_profile(
        db: &Arc<DatabaseConnection>,
        user_id: i32,
    ) -> Result<UserResponse, ApiError> {
        let user = users::Entity::find_by_id(user_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;

        Ok(UserResponse::from(user))
    }
}
