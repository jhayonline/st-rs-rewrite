use crate::entities::users;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,
    pub career_path: Option<String>,
    pub specialization: Option<String>,
    pub community_link: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub full_name: String,
    pub role: String,
    pub membership_category: String,
    pub career_path: Option<String>,
    pub specialization: Option<String>,
    pub status: String,
    pub membership_enabled: bool,
    pub membership_paid: bool,
    pub contract_file_url: Option<String>,
    pub community_link: Option<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<users::Model> for UserResponse {
    fn from(user: users::Model) -> Self {
        Self {
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
        }
    }
}
