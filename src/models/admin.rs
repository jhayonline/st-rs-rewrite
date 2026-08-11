use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GetUsersRequest {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub membership_category: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub full_name: String,
    pub email: String,
    pub password: String,
    pub membership_category: String,
    pub career_path: Option<String>,
    pub role: String,
    pub status: Option<String>,
    pub membership_enabled: Option<bool>,
    pub membership_amount: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserRequest {
    pub user_id: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub membership_category: Option<String>,
    pub career_path: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub community_link: Option<String>,
    pub membership_enabled: Option<bool>,
    pub membership_amount: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteUserRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateMentorAssignmentsRequest {
    pub mentee_id: String,
    pub assignments: Vec<MentorAssignment>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MentorAssignment {
    pub id: Option<String>,
    pub mentor: String,
}

// Response models
#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserAdminResponse>,
    pub total_count: i64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}

#[derive(Debug, Serialize)]
pub struct UserAdminResponse {
    pub id: i32,
    pub email: String,
    pub full_name: String,
    pub membership_category: String,
    pub career_path: Option<String>,
    pub role: String,
    pub status: String,
    pub specialization: Option<String>,
    pub contract_file_url: Option<String>,
    pub membership_enabled: bool,
    pub membership_amount: Option<String>,
    pub membership_paid: bool,
    pub payment_reference: Option<String>,
    pub payment_date: Option<DateTime<FixedOffset>>,
    pub community_link: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total: i64,
    pub approved: i64,
    pub pending: i64,
    pub rejected: i64,
    pub suspended: i64,
}

#[derive(Debug, Serialize)]
pub struct MentorResponse {
    pub id: String,
    pub full_name: String,
    pub specialization: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MentorAssignmentsResponse {
    pub assignments: Vec<MentorAssignmentDetail>,
}

#[derive(Debug, Serialize)]
pub struct MentorAssignmentDetail {
    pub id: String,
    pub mentor: String,
    pub mentor_name: String,
}
