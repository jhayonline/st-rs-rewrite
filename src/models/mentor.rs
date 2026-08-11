use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

// ============= Course Models =============
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCourseRequest {
    pub name: String,
    pub duration: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateCourseRequest {
    pub course_id: Option<i32>,
    pub name: Option<String>,
    pub duration: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CourseResponse {
    pub id: i32,
    pub mentor_id: i32,
    pub name: String,
    pub duration: String,
    pub description: String,
    pub status: String,
    pub enrolled_mentees_count: i32,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize)]
pub struct CourseDetailResponse {
    pub id: i32,
    pub name: String,
    pub duration: String,
    pub description: String,
    pub status: String,
    pub enrolled_mentees: i32,
    pub created_at: DateTime<FixedOffset>,
    pub lessons: Vec<LessonResponse>,
    pub tasks: Vec<TaskResponse>,
}

// ============= Lesson Models =============
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateLessonRequest {
    pub course_id: i32,
    pub title: String,
    pub description: String,
    pub link: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateLessonRequest {
    pub lesson_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LessonResponse {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub link: String,
    pub order_index: i32,
    pub status: String,
    pub created_at: DateTime<FixedOffset>,
}

// ============= Task Models =============
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTaskRequest {
    pub course_id: i32,
    pub title: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub deadline: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTaskRequest {
    pub task_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub requirements: Option<Vec<String>>,
    pub deadline: Option<DateTime<FixedOffset>>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub deadline: Option<DateTime<FixedOffset>>,
    pub status: String,
    pub created_at: DateTime<FixedOffset>,
}

// ============= Mentee Management Models =============
#[derive(Debug, Serialize)]
pub struct AssignedMenteeResponse {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub career_path: Option<String>,
    pub membership_category: String,
    pub is_enrolled: bool,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct MenteeDetailResponse {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub membership_category: String,
    pub career_path: Option<String>,
    pub status: String,
    pub progress_percentage: i32,
    pub contract_file_url: Option<String>,
    pub community_link: Option<String>,
    pub joined_date: DateTime<FixedOffset>,
    pub last_active: DateTime<FixedOffset>,
    pub notes: Option<String>,
    pub courses: Vec<MenteeCourseProgress>,
}

#[derive(Debug, Serialize)]
pub struct MenteeCourseProgress {
    pub id: String,
    pub name: String,
    pub duration: String,
    pub description: String,
    pub status: String,
    pub progress: i32,
    pub completed_lessons: i32,
    pub total_lessons: i32,
    pub approved_tasks: i32,
    pub total_tasks: i32,
    pub enrolled_date: DateTime<FixedOffset>,
    pub completed_at: Option<DateTime<FixedOffset>>,
}

// ============= Enrollment Models =============
#[derive(Debug, Deserialize, Serialize)]
pub struct EnrollMenteesRequest {
    pub course_id: i32,
    pub mentee_ids: Vec<String>, // UUID strings
}

#[derive(Debug, Serialize)]
pub struct EnrollResponse {
    pub enrolled_count: i32,
    pub total_enrolled: i32,
    pub already_enrolled_count: i32,
}

// ============= Submission Models =============
#[derive(Debug, Serialize)]
pub struct SubmissionResponse {
    pub id: i32,
    pub task_id: i32,
    pub task_title: String,
    pub task_description: String,
    pub deadline: Option<DateTime<FixedOffset>>,
    pub mentee_id: i32,
    pub mentee_name: String,
    pub mentee_email: String,
    pub course_id: i32,
    pub course_name: String,
    pub submission_link: Option<String>,
    pub submission_notes: Option<String>,
    pub submitted_at: DateTime<FixedOffset>,
    pub status: String,
    pub mentor_feedback: Option<String>,
    pub reviewed_at: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewSubmissionRequest {
    pub submission_id: i32,
    pub status: String, // approved or rejected
    pub feedback: Option<String>,
}

// ============= Dashboard Models =============
#[derive(Debug, Serialize)]
pub struct MentorDashboardResponse {
    pub stats: MentorStats,
    pub recent_courses: Vec<CourseResponse>,
}

#[derive(Debug, Serialize)]
pub struct MentorStats {
    pub total_mentees: i64,
    pub active_mentees: i64,
    pub total_courses: i64,
    pub pending_submissions: i64,
}
