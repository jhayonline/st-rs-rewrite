use serde::{Deserialize, Serialize};

use chrono::{DateTime, FixedOffset};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub membership_category: String, // Student, Professional, Volunteer
    pub role: Option<String>,        // Mentee or Mentor
    pub career_path: Option<String>,
    pub specialization: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub profile: UserProfile,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserProfile {
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
    pub created_at: DateTime<FixedOffset>,
}
