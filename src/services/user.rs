use crate::entities::users;
use crate::models::user::{ChangePasswordRequest, UpdateProfileRequest, UserResponse};
use crate::utils::error::ApiError;
use crate::utils::validation;

use bcrypt::{DEFAULT_COST, hash, verify};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};
use std::sync::Arc;

pub struct UserService;

impl UserService {
    pub async fn update_profile(
        db: &Arc<DatabaseConnection>,
        user_id: i32,
        req: &UpdateProfileRequest,
    ) -> Result<UserResponse, ApiError> {
        let user = users::Entity::find_by_id(user_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;

        // Build update data
        let mut active_user: users::ActiveModel = user.into();

        if let Some(full_name) = &req.full_name {
            if full_name.len() < 2 || full_name.len() > 100 {
                return Err(ApiError::bad_request(
                    "Full name must be between 2 and 100 characters",
                ));
            }
            active_user.name = ActiveValue::Set(full_name.clone());
        }

        if let Some(career_path) = &req.career_path {
            active_user.career_path = ActiveValue::Set(Some(career_path.clone()));
        }

        if let Some(specialization) = &req.specialization {
            active_user.specialization = ActiveValue::Set(Some(specialization.clone()));
        }

        if let Some(community_link) = &req.community_link {
            // Basic URL validation
            if !community_link.starts_with("http://") && !community_link.starts_with("https://") {
                return Err(ApiError::bad_request("Community link must be a valid URL"));
            }
            active_user.community_link = ActiveValue::Set(Some(community_link.clone()));
        }

        // Save changes
        let updated_user = active_user.update(db.as_ref()).await?;
        Ok(UserResponse::from(updated_user))
    }

    pub async fn change_password(
        db: &Arc<DatabaseConnection>,
        user_id: i32,
        req: &ChangePasswordRequest,
    ) -> Result<(), ApiError> {
        let user = users::Entity::find_by_id(user_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;

        // Validate current password
        if !verify(&req.current_password, &user.password)? {
            return Err(ApiError::unauthorized("Current password is incorrect"));
        }

        // Validate new password
        if let Err(e) = validation::validate_password(&req.new_password) {
            return Err(ApiError::bad_request(e));
        }

        // Hash new password
        let new_password_hash = hash(&req.new_password, DEFAULT_COST)?;

        // Update password
        let mut active_user: users::ActiveModel = user.into();
        active_user.password = ActiveValue::Set(new_password_hash);
        active_user.update(db.as_ref()).await?;

        Ok(())
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
