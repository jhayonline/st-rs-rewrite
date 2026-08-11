use crate::middleware::auth::get_user_id;
use crate::models::mentor::{
    CreateCourseRequest, CreateLessonRequest, CreateTaskRequest, EnrollMenteesRequest,
    ReviewSubmissionRequest, UpdateCourseRequest, UpdateLessonRequest, UpdateTaskRequest,
};
use crate::services::mentor::MentorService;
use crate::utils::error::ApiError;
use salvo::prelude::*;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// ============= Course Controllers =============

#[handler]
pub async fn create_course(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let create_req: CreateCourseRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let course = MentorService::create_course(&db, mentor_id, &create_req).await?;
    Ok(Json(serde_json::to_value(course).unwrap()))
}

#[handler]
pub async fn get_courses(depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let courses = MentorService::get_courses(&db, mentor_id).await?;
    Ok(Json(serde_json::to_value(courses).unwrap()))
}

#[handler]
pub async fn get_course_detail(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Use params() for query parameters
    let params = req.params();
    let course_id = req
        .query::<i32>("courseId")
        .ok_or_else(|| ApiError::bad_request("courseId is required"))?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let course = MentorService::get_course_detail(&db, course_id, mentor_id).await?;
    Ok(Json(serde_json::to_value(course).unwrap()))
}

#[handler]
pub async fn update_course_status(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let update_req: UpdateCourseRequest = req.parse_json().await?;
    let course_id = update_req
        .course_id
        .ok_or_else(|| ApiError::bad_request("course_id is required"))?;
    let status = update_req
        .status
        .ok_or_else(|| ApiError::bad_request("status is required"))?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let course = MentorService::update_course_status(&db, course_id, mentor_id, &status).await?;
    Ok(Json(serde_json::to_value(course).unwrap()))
}

// ============= Lesson Controllers =============

#[handler]
pub async fn create_lesson(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let create_req: CreateLessonRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let lesson = MentorService::create_lesson(&db, mentor_id, &create_req).await?;
    Ok(Json(serde_json::to_value(lesson).unwrap()))
}

#[handler]
pub async fn update_lesson(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let update_req: UpdateLessonRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let lesson = MentorService::update_lesson(&db, mentor_id, &update_req).await?;
    Ok(Json(serde_json::to_value(lesson).unwrap()))
}

#[handler]
pub async fn delete_lesson(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let lesson_id = req
        .query::<i32>("lessonId")
        .ok_or_else(|| ApiError::bad_request("lessonId is required"))?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    MentorService::delete_lesson(&db, lesson_id, mentor_id).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Lesson deleted successfully"
    })))
}

// ============= Task Controllers =============

#[handler]
pub async fn create_task(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let create_req: CreateTaskRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let task = MentorService::create_task(&db, mentor_id, &create_req).await?;
    Ok(Json(serde_json::to_value(task).unwrap()))
}

#[handler]
pub async fn update_task(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let update_req: UpdateTaskRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let task = MentorService::update_task(&db, mentor_id, &update_req).await?;
    Ok(Json(serde_json::to_value(task).unwrap()))
}

#[handler]
pub async fn delete_task(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let task_id = req
        .query::<i32>("taskId")
        .ok_or_else(|| ApiError::bad_request("taskId is required"))?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    MentorService::delete_task(&db, task_id, mentor_id).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Task deleted successfully"
    })))
}
// ============= Mentee Management Controllers =============

#[handler]
pub async fn get_assigned_mentees(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let params = req.params();
    let course_id = params.get("courseId").and_then(|id| id.parse::<i32>().ok());

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let mentees = MentorService::get_assigned_mentees(&db, mentor_id, course_id).await?;
    Ok(Json(serde_json::to_value(mentees).unwrap()))
}

#[handler]
pub async fn get_mentee_detail(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let params = req.params();
    let mentee_id = params
        .get("menteeId")
        .ok_or_else(|| ApiError::bad_request("menteeId is required"))?;

    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let mentee = MentorService::get_mentee_detail(&db, mentor_id, mentee_id).await?;
    Ok(Json(serde_json::to_value(mentee).unwrap()))
}

// ============= Enrollment Controllers =============

#[handler]
pub async fn enroll_mentees(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let enroll_req: EnrollMenteesRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let result = MentorService::enroll_mentees(&db, mentor_id, &enroll_req).await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

// ============= Submission Controllers =============

#[handler]
pub async fn get_submissions(depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let submissions = MentorService::get_submissions(&db, mentor_id).await?;
    Ok(Json(serde_json::to_value(submissions).unwrap()))
}

#[handler]
pub async fn review_submission(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<serde_json::Value>, ApiError> {
    let review_req: ReviewSubmissionRequest = req.parse_json().await?;
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let submission = MentorService::review_submission(&db, mentor_id, &review_req).await?;
    Ok(Json(serde_json::to_value(submission).unwrap()))
}

// ============= Dashboard Controller =============

#[handler]
pub async fn get_dashboard(depot: &mut Depot) -> Result<Json<serde_json::Value>, ApiError> {
    let db = depot
        .get_typed::<Arc<DatabaseConnection>>()
        .map_err(|_| ApiError::internal_error("Database connection not found"))?;
    let mentor_id =
        get_user_id(depot).ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    let dashboard = MentorService::get_dashboard(&db, mentor_id).await?;
    Ok(Json(serde_json::to_value(dashboard).unwrap()))
}
