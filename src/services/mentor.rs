use crate::entities::{
    course_enrollments, courses, lesson_progress, lessons, mentor_mentee_relationships,
    task_submissions, tasks, users,
};
use crate::models::mentor::{
    AssignedMenteeResponse, CourseDetailResponse, CourseResponse, CreateCourseRequest,
    CreateLessonRequest, CreateTaskRequest, EnrollMenteesRequest, EnrollResponse, LessonResponse,
    MenteeCourseProgress, MenteeDetailResponse, MentorDashboardResponse, MentorStats,
    ReviewSubmissionRequest, SubmissionResponse, TaskResponse, UpdateLessonRequest,
    UpdateTaskRequest,
};
use crate::utils::error::ApiError;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct MentorService;

impl MentorService {
    // ============= Course Management =============

    /// Create a new course
    pub async fn create_course(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &CreateCourseRequest,
    ) -> Result<CourseResponse, ApiError> {
        // Validate input
        if req.name.trim().is_empty() {
            return Err(ApiError::bad_request("Course name cannot be empty"));
        }
        if req.description.trim().is_empty() {
            return Err(ApiError::bad_request("Course description cannot be empty"));
        }

        // Check if course name already exists for this mentor
        let existing = courses::Entity::find()
            .filter(courses::Column::MentorId.eq(mentor_id))
            .filter(courses::Column::Name.eq(&req.name))
            .one(db.as_ref())
            .await?;

        if existing.is_some() {
            return Err(ApiError::conflict("Course with this name already exists"));
        }

        let course = courses::ActiveModel {
            mentor_id: ActiveValue::Set(mentor_id),
            name: ActiveValue::Set(req.name.clone()),
            duration: ActiveValue::Set(req.duration.clone()),
            description: ActiveValue::Set(req.description.clone()),
            status: ActiveValue::Set("active".to_string()),
            enrolled_mentees_count: ActiveValue::Set(Some(0)),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await?;

        Ok(CourseResponse {
            id: course.id,
            mentor_id: course.mentor_id,
            name: course.name,
            duration: course.duration,
            description: course.description,
            status: course.status,
            enrolled_mentees_count: course.enrolled_mentees_count.unwrap_or(0),
            created_at: course.created_at,
            updated_at: course.updated_at,
        })
    }

    /// Get all courses for a mentor
    pub async fn get_courses(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
    ) -> Result<Vec<CourseResponse>, ApiError> {
        let courses = courses::Entity::find()
            .filter(courses::Column::MentorId.eq(mentor_id))
            .order_by_desc(courses::Column::CreatedAt)
            .all(db.as_ref())
            .await?;

        Ok(courses
            .into_iter()
            .map(|c| CourseResponse {
                id: c.id,
                mentor_id: c.mentor_id,
                name: c.name,
                duration: c.duration,
                description: c.description,
                status: c.status,
                enrolled_mentees_count: c.enrolled_mentees_count.unwrap_or(0),
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect())
    }

    /// Get course details with lessons and tasks
    pub async fn get_course_detail(
        db: &Arc<DatabaseConnection>,
        course_id: i32,
        mentor_id: i32,
    ) -> Result<CourseDetailResponse, ApiError> {
        // Verify course belongs to mentor
        let course = courses::Entity::find()
            .filter(courses::Column::Id.eq(course_id))
            .filter(courses::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Course not found"))?;

        // Get lessons
        let lesson_entities = lessons::Entity::find()
            .filter(lessons::Column::CourseId.eq(course_id))
            .order_by_asc(lessons::Column::OrderIndex)
            .all(db.as_ref())
            .await?;

        let lessons_response: Vec<LessonResponse> = lesson_entities
            .into_iter()
            .map(|l| LessonResponse {
                id: l.id,
                title: l.title,
                description: l.description,
                link: l.link,
                order_index: l.order_index,
                status: l.status,
                created_at: l.created_at,
            })
            .collect();

        // Get tasks
        let task_entities = tasks::Entity::find()
            .filter(tasks::Column::CourseId.eq(course_id))
            .order_by_desc(tasks::Column::CreatedAt)
            .all(db.as_ref())
            .await?;

        let tasks_response: Vec<TaskResponse> = task_entities
            .into_iter()
            .map(|t| {
                let requirements = t
                    .requirements
                    .and_then(|json| serde_json::from_value::<Vec<String>>(json).ok())
                    .unwrap_or_default();
                TaskResponse {
                    id: t.id,
                    title: t.title,
                    description: t.description,
                    requirements,
                    deadline: t.deadline,
                    status: t.status,
                    created_at: t.created_at,
                }
            })
            .collect();

        Ok(CourseDetailResponse {
            id: course.id,
            name: course.name,
            duration: course.duration,
            description: course.description,
            status: course.status,
            enrolled_mentees: course.enrolled_mentees_count.unwrap_or(0),
            created_at: course.created_at,
            lessons: lessons_response,
            tasks: tasks_response,
        })
    }

    /// Update course status
    pub async fn update_course_status(
        db: &Arc<DatabaseConnection>,
        course_id: i32,
        mentor_id: i32,
        status: &str,
    ) -> Result<CourseResponse, ApiError> {
        let valid_statuses = vec!["active", "archived", "ended"];
        if !valid_statuses.contains(&status) {
            return Err(ApiError::bad_request(format!(
                "Invalid status. Must be one of: {}",
                valid_statuses.join(", ")
            )));
        }

        let course = courses::Entity::find()
            .filter(courses::Column::Id.eq(course_id))
            .filter(courses::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Course not found"))?;

        let mut active_course: courses::ActiveModel = course.into();
        active_course.status = ActiveValue::Set(status.to_string());

        let updated = active_course.update(db.as_ref()).await?;

        Ok(CourseResponse {
            id: updated.id,
            mentor_id: updated.mentor_id,
            name: updated.name,
            duration: updated.duration,
            description: updated.description,
            status: updated.status,
            enrolled_mentees_count: updated.enrolled_mentees_count.unwrap_or(0),
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        })
    }

    // ============= Lesson Management =============

    /// Create a new lesson
    pub async fn create_lesson(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &CreateLessonRequest,
    ) -> Result<LessonResponse, ApiError> {
        // Verify course belongs to mentor
        let _course = courses::Entity::find()
            .filter(courses::Column::Id.eq(req.course_id))
            .filter(courses::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Course not found"))?;

        // Get next order index
        let max_order = lessons::Entity::find()
            .filter(lessons::Column::CourseId.eq(req.course_id))
            .order_by_desc(lessons::Column::OrderIndex)
            .one(db.as_ref())
            .await?;

        let order_index = max_order.map(|l| l.order_index + 1).unwrap_or(0);

        let lesson = lessons::ActiveModel {
            course_id: ActiveValue::Set(req.course_id),
            mentor_id: ActiveValue::Set(mentor_id),
            title: ActiveValue::Set(req.title.clone()),
            description: ActiveValue::Set(req.description.clone()),
            link: ActiveValue::Set(req.link.clone()),
            order_index: ActiveValue::Set(order_index),
            status: ActiveValue::Set("active".to_string()),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await?;

        Ok(LessonResponse {
            id: lesson.id,
            title: lesson.title,
            description: lesson.description,
            link: lesson.link,
            order_index: lesson.order_index,
            status: lesson.status,
            created_at: lesson.created_at,
        })
    }

    /// Update a lesson
    pub async fn update_lesson(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &UpdateLessonRequest,
    ) -> Result<LessonResponse, ApiError> {
        let lesson = lessons::Entity::find()
            .filter(lessons::Column::Id.eq(req.lesson_id))
            .filter(lessons::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Lesson not found"))?;

        let mut active_lesson: lessons::ActiveModel = lesson.into();

        if let Some(title) = &req.title {
            active_lesson.title = ActiveValue::Set(title.clone());
        }
        if let Some(description) = &req.description {
            active_lesson.description = ActiveValue::Set(description.clone());
        }
        if let Some(link) = &req.link {
            active_lesson.link = ActiveValue::Set(link.clone());
        }

        let updated = active_lesson.update(db.as_ref()).await?;

        Ok(LessonResponse {
            id: updated.id,
            title: updated.title,
            description: updated.description,
            link: updated.link,
            order_index: updated.order_index,
            status: updated.status,
            created_at: updated.created_at,
        })
    }

    /// Delete a lesson
    pub async fn delete_lesson(
        db: &Arc<DatabaseConnection>,
        lesson_id: i32,
        mentor_id: i32,
    ) -> Result<(), ApiError> {
        let lesson = lessons::Entity::find()
            .filter(lessons::Column::Id.eq(lesson_id))
            .filter(lessons::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Lesson not found"))?;

        let active_lesson: lessons::ActiveModel = lesson.into();
        active_lesson.delete(db.as_ref()).await?;

        Ok(())
    }

    // ============= Task Management =============

    /// Create a new task
    pub async fn create_task(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &CreateTaskRequest,
    ) -> Result<TaskResponse, ApiError> {
        // Verify course belongs to mentor
        let _course = courses::Entity::find()
            .filter(courses::Column::Id.eq(req.course_id))
            .filter(courses::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Course not found"))?;

        // Filter empty requirements
        let requirements: Vec<String> = req
            .requirements
            .iter()
            .filter(|r| !r.trim().is_empty())
            .cloned()
            .collect();

        // Clone requirements for later use
        let requirements_clone = requirements.clone();
        let requirements_json = serde_json::to_value(&requirements).ok();

        let task = tasks::ActiveModel {
            course_id: ActiveValue::Set(req.course_id),
            mentor_id: ActiveValue::Set(mentor_id),
            title: ActiveValue::Set(req.title.clone()),
            description: ActiveValue::Set(req.description.clone()),
            requirements: ActiveValue::Set(requirements_json),
            deadline: ActiveValue::Set(req.deadline),
            status: ActiveValue::Set("active".to_string()),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await?;

        Ok(TaskResponse {
            id: task.id,
            title: task.title,
            description: task.description,
            requirements: requirements_clone,
            deadline: task.deadline,
            status: task.status,
            created_at: task.created_at,
        })
    }

    /// Update a task
    pub async fn update_task(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &UpdateTaskRequest,
    ) -> Result<TaskResponse, ApiError> {
        let task = tasks::Entity::find()
            .filter(tasks::Column::Id.eq(req.task_id))
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Task not found"))?;

        let mut active_task: tasks::ActiveModel = task.into();

        if let Some(title) = &req.title {
            active_task.title = ActiveValue::Set(title.clone());
        }
        if let Some(description) = &req.description {
            active_task.description = ActiveValue::Set(description.clone());
        }
        if let Some(requirements) = &req.requirements {
            let filtered: Vec<String> = requirements
                .iter()
                .filter(|r| !r.trim().is_empty())
                .cloned()
                .collect();
            let json = serde_json::to_value(&filtered).ok();
            active_task.requirements = ActiveValue::Set(json);
        }
        if let Some(deadline) = &req.deadline {
            active_task.deadline = ActiveValue::Set(Some(*deadline));
        }
        if let Some(status) = &req.status {
            active_task.status = ActiveValue::Set(status.clone());
        }

        let updated = active_task.update(db.as_ref()).await?;

        let requirements = updated
            .requirements
            .clone()
            .and_then(|json| serde_json::from_value::<Vec<String>>(json).ok())
            .unwrap_or_default();

        Ok(TaskResponse {
            id: updated.id,
            title: updated.title,
            description: updated.description,
            requirements,
            deadline: updated.deadline,
            status: updated.status,
            created_at: updated.created_at,
        })
    }

    /// Delete a task
    pub async fn delete_task(
        db: &Arc<DatabaseConnection>,
        task_id: i32,
        mentor_id: i32,
    ) -> Result<(), ApiError> {
        let task = tasks::Entity::find()
            .filter(tasks::Column::Id.eq(task_id))
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Task not found"))?;

        let active_task: tasks::ActiveModel = task.into();
        active_task.delete(db.as_ref()).await?;

        Ok(())
    }

    // ============= Mentee Management =============

    /// Get assigned mentees for a mentor
    pub async fn get_assigned_mentees(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        course_id: Option<i32>,
    ) -> Result<Vec<AssignedMenteeResponse>, ApiError> {
        let relationships = mentor_mentee_relationships::Entity::find()
            .filter(mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(mentor_mentee_relationships::Column::Status.eq("active"))
            .all(db.as_ref())
            .await?;

        let mut result = Vec::new();

        for rel in relationships {
            let mentee = users::Entity::find_by_id(rel.mentee_id)
                .one(db.as_ref())
                .await?;

            if let Some(mentee) = mentee {
                let is_enrolled = if let Some(course_id) = course_id {
                    course_enrollments::Entity::find()
                        .filter(course_enrollments::Column::CourseId.eq(course_id))
                        .filter(course_enrollments::Column::MenteeId.eq(mentee.id))
                        .count(db.as_ref())
                        .await?
                        > 0
                } else {
                    false
                };

                result.push(AssignedMenteeResponse {
                    id: mentee.pid.to_string(),
                    full_name: mentee.name,
                    email: mentee.email,
                    career_path: mentee.career_path,
                    membership_category: mentee.membership_category,
                    is_enrolled,
                    status: rel.status,
                });
            }
        }

        Ok(result)
    }

    /// Get mentee detail with course progress
    pub async fn get_mentee_detail(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        mentee_id: &str,
    ) -> Result<MenteeDetailResponse, ApiError> {
        let mentee_pid = Uuid::parse_str(mentee_id)
            .map_err(|_| ApiError::bad_request("Invalid mentee ID format"))?;

        // Verify relationship exists
        let mentee = users::Entity::find()
            .filter(users::Column::Pid.eq(mentee_pid))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Mentee not found"))?;

        let relationship = mentor_mentee_relationships::Entity::find()
            .filter(mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(mentor_mentee_relationships::Column::MenteeId.eq(mentee.id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Mentee not assigned to you"))?;

        // Get courses with progress
        let enrollments = course_enrollments::Entity::find()
            .filter(course_enrollments::Column::MenteeId.eq(mentee.id))
            .all(db.as_ref())
            .await?;

        let mut courses_progress = Vec::new();

        for enrollment in enrollments {
            let course = courses::Entity::find_by_id(enrollment.course_id)
                .one(db.as_ref())
                .await?;

            if let Some(course) = course {
                // Get lesson stats
                let total_lessons = lessons::Entity::find()
                    .filter(lessons::Column::CourseId.eq(course.id))
                    .filter(lessons::Column::Status.eq("active"))
                    .count(db.as_ref())
                    .await?
                    .try_into()
                    .unwrap_or(0);

                // Get completed lessons (using raw query approach)
                let completed_lessons = lesson_progress::Entity::find()
                    .filter(lesson_progress::Column::MenteeId.eq(mentee.id))
                    .filter(lesson_progress::Column::Completed.eq(true))
                    .all(db.as_ref())
                    .await?;

                // Filter completed lessons that belong to this course
                let mut completed_count = 0;
                for progress in completed_lessons {
                    let lesson = lessons::Entity::find_by_id(progress.lesson_id)
                        .one(db.as_ref())
                        .await?;
                    if let Some(lesson) = lesson {
                        if lesson.course_id == course.id {
                            completed_count += 1;
                        }
                    }
                }

                // Get task stats
                let total_tasks = tasks::Entity::find()
                    .filter(tasks::Column::CourseId.eq(course.id))
                    .filter(tasks::Column::Status.eq("active"))
                    .count(db.as_ref())
                    .await?
                    .try_into()
                    .unwrap_or(0);

                // Get approved tasks
                let approved_tasks = task_submissions::Entity::find()
                    .filter(task_submissions::Column::MenteeId.eq(mentee.id))
                    .filter(task_submissions::Column::Status.eq("approved"))
                    .all(db.as_ref())
                    .await?;

                let mut approved_count = 0;
                for submission in approved_tasks {
                    let task = tasks::Entity::find_by_id(submission.task_id)
                        .one(db.as_ref())
                        .await?;
                    if let Some(task) = task {
                        if task.course_id == course.id {
                            approved_count += 1;
                        }
                    }
                }

                courses_progress.push(MenteeCourseProgress {
                    id: course.id.to_string(),
                    name: course.name,
                    duration: course.duration,
                    description: course.description,
                    status: enrollment.status,
                    progress: enrollment.progress_percentage.unwrap_or(0),
                    completed_lessons: completed_count,
                    total_lessons,
                    approved_tasks: approved_count,
                    total_tasks,
                    enrolled_date: enrollment.enrolled_at.unwrap_or(enrollment.created_at),
                    completed_at: enrollment.completed_at,
                });
            }
        }

        Ok(MenteeDetailResponse {
            id: mentee.pid.to_string(),
            full_name: mentee.name,
            email: mentee.email,
            membership_category: mentee.membership_category,
            career_path: mentee.career_path,
            status: relationship.status,
            progress_percentage: relationship.progress_percentage.unwrap_or(0),
            contract_file_url: mentee.contract_file_url,
            community_link: mentee.community_link,
            joined_date: relationship.assigned_date,
            last_active: relationship.updated_at,
            notes: relationship.notes,
            courses: courses_progress,
        })
    }

    // ============= Enrollment =============

    /// Enroll mentees in a course
    pub async fn enroll_mentees(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &EnrollMenteesRequest,
    ) -> Result<EnrollResponse, ApiError> {
        // Verify course belongs to mentor
        let course = courses::Entity::find()
            .filter(courses::Column::Id.eq(req.course_id))
            .filter(courses::Column::MentorId.eq(mentor_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Course not found"))?;

        // Get all mentees
        let mut enrolled_count = 0;
        let mut already_enrolled = 0;

        for mentee_id_str in &req.mentee_ids {
            let mentee_pid = Uuid::parse_str(mentee_id_str)
                .map_err(|_| ApiError::bad_request("Invalid mentee ID format"))?;

            let mentee = users::Entity::find()
                .filter(users::Column::Pid.eq(mentee_pid))
                .one(db.as_ref())
                .await?
                .ok_or_else(|| {
                    ApiError::not_found(format!("Mentee not found: {}", mentee_id_str))
                })?;

            // Verify mentee is assigned to this mentor
            let relationship = mentor_mentee_relationships::Entity::find()
                .filter(mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
                .filter(mentor_mentee_relationships::Column::MenteeId.eq(mentee.id))
                .one(db.as_ref())
                .await?;

            if relationship.is_none() {
                return Err(ApiError::bad_request(format!(
                    "Mentee {} is not assigned to you",
                    mentee_id_str
                )));
            }

            // Check if already enrolled
            let existing = course_enrollments::Entity::find()
                .filter(course_enrollments::Column::CourseId.eq(req.course_id))
                .filter(course_enrollments::Column::MenteeId.eq(mentee.id))
                .one(db.as_ref())
                .await?;

            if existing.is_some() {
                already_enrolled += 1;
                continue;
            }

            // Enroll the mentee
            course_enrollments::ActiveModel {
                course_id: ActiveValue::Set(req.course_id),
                mentee_id: ActiveValue::Set(mentee.id),
                status: ActiveValue::Set("active".to_string()),
                progress_percentage: ActiveValue::Set(Some(0)),
                ..Default::default()
            }
            .insert(db.as_ref())
            .await?;

            enrolled_count += 1;
        }

        // Update course enrollment count
        let mut active_course: courses::ActiveModel = course.into();
        let current_count = match active_course.enrolled_mentees_count.as_ref() {
            Some(val) => *val,
            None => 0,
        };
        active_course.enrolled_mentees_count =
            ActiveValue::Set(Some(current_count + enrolled_count));
        active_course.update(db.as_ref()).await?;

        Ok(EnrollResponse {
            enrolled_count,
            total_enrolled: enrolled_count + already_enrolled,
            already_enrolled_count: already_enrolled,
        })
    }

    // ============= Submission Management =============

    /// Get submissions for a mentor
    pub async fn get_submissions(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
    ) -> Result<Vec<SubmissionResponse>, ApiError> {
        let submissions = task_submissions::Entity::find().all(db.as_ref()).await?;

        let mut result = Vec::new();

        for submission in submissions {
            let task = tasks::Entity::find_by_id(submission.task_id)
                .one(db.as_ref())
                .await?;

            // Skip if task doesn't belong to this mentor
            if let Some(task) = &task {
                if task.mentor_id != mentor_id {
                    continue;
                }
            } else {
                continue;
            }

            let mentee = users::Entity::find_by_id(submission.mentee_id)
                .one(db.as_ref())
                .await?;

            let course = if let Some(task) = &task {
                courses::Entity::find_by_id(task.course_id)
                    .one(db.as_ref())
                    .await?
            } else {
                None
            };

            if let (Some(task), Some(mentee), Some(course)) = (task, mentee, course) {
                result.push(SubmissionResponse {
                    id: submission.id,
                    task_id: task.id,
                    task_title: task.title,
                    task_description: task.description,
                    deadline: task.deadline,
                    mentee_id: mentee.id,
                    mentee_name: mentee.name,
                    mentee_email: mentee.email,
                    course_id: course.id,
                    course_name: course.name,
                    submission_link: submission.submission_link,
                    submission_notes: submission.submission_notes,
                    submitted_at: submission.submitted_at.unwrap_or(submission.created_at),
                    status: submission.status,
                    mentor_feedback: submission.mentor_feedback,
                    reviewed_at: submission.reviewed_at,
                });
            }
        }

        Ok(result)
    }

    /// Review a submission
    pub async fn review_submission(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
        req: &ReviewSubmissionRequest,
    ) -> Result<SubmissionResponse, ApiError> {
        if !["approved", "rejected"].contains(&req.status.as_str()) {
            return Err(ApiError::bad_request(
                "Status must be 'approved' or 'rejected'",
            ));
        }

        if req.status == "rejected" && req.feedback.is_none() {
            return Err(ApiError::bad_request(
                "Feedback is required for rejected submissions",
            ));
        }

        // Get the submission
        let submission = task_submissions::Entity::find()
            .filter(task_submissions::Column::Id.eq(req.submission_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Submission not found"))?;

        // Verify task belongs to mentor
        let task = tasks::Entity::find_by_id(submission.task_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Task not found"))?;

        if task.mentor_id != mentor_id {
            return Err(ApiError::forbidden(
                "You are not authorized to review this submission",
            ));
        }

        // Update the submission
        let mut active_submission: task_submissions::ActiveModel = submission.into();
        active_submission.status = ActiveValue::Set(req.status.clone());
        active_submission.mentor_feedback = ActiveValue::Set(req.feedback.clone());
        active_submission.reviewed_at = ActiveValue::Set(Some(chrono::Utc::now().into()));

        let updated = active_submission.update(db.as_ref()).await?;

        // Get the mentee
        let mentee = users::Entity::find_by_id(updated.mentee_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Mentee not found"))?;

        // Get the course
        let course = courses::Entity::find_by_id(task.course_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Course not found"))?;

        Ok(SubmissionResponse {
            id: updated.id,
            task_id: task.id,
            task_title: task.title,
            task_description: task.description,
            deadline: task.deadline,
            mentee_id: mentee.id,
            mentee_name: mentee.name,
            mentee_email: mentee.email,
            course_id: course.id,
            course_name: course.name,
            submission_link: updated.submission_link,
            submission_notes: updated.submission_notes,
            submitted_at: updated.submitted_at.unwrap_or(updated.created_at),
            status: updated.status,
            mentor_feedback: updated.mentor_feedback,
            reviewed_at: updated.reviewed_at,
        })
    }

    // ============= Dashboard =============

    /// Get mentor dashboard statistics
    pub async fn get_dashboard(
        db: &Arc<DatabaseConnection>,
        mentor_id: i32,
    ) -> Result<MentorDashboardResponse, ApiError> {
        // Total mentees
        let total_mentees = mentor_mentee_relationships::Entity::find()
            .filter(mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        // Active mentees
        let active_mentees = mentor_mentee_relationships::Entity::find()
            .filter(mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(mentor_mentee_relationships::Column::Status.eq("active"))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        // Total courses
        let total_courses = courses::Entity::find()
            .filter(courses::Column::MentorId.eq(mentor_id))
            .filter(courses::Column::Status.eq("active"))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        // Pending submissions
        let all_submissions = task_submissions::Entity::find().all(db.as_ref()).await?;

        let mut pending_submissions = 0;
        for submission in all_submissions {
            if submission.status == "pending" || submission.status == "submitted" {
                let task = tasks::Entity::find_by_id(submission.task_id)
                    .one(db.as_ref())
                    .await?;
                if let Some(task) = task {
                    if task.mentor_id == mentor_id {
                        pending_submissions += 1;
                    }
                }
            }
        }

        // Recent courses
        let recent_courses = courses::Entity::find()
            .filter(courses::Column::MentorId.eq(mentor_id))
            .filter(courses::Column::Status.eq("active"))
            .order_by_desc(courses::Column::CreatedAt)
            .limit(10)
            .all(db.as_ref())
            .await?;

        let course_responses: Vec<CourseResponse> = recent_courses
            .into_iter()
            .map(|c| CourseResponse {
                id: c.id,
                mentor_id: c.mentor_id,
                name: c.name,
                duration: c.duration,
                description: c.description,
                status: c.status,
                enrolled_mentees_count: c.enrolled_mentees_count.unwrap_or(0),
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect();

        Ok(MentorDashboardResponse {
            stats: MentorStats {
                total_mentees,
                active_mentees,
                total_courses,
                pending_submissions: pending_submissions.try_into().unwrap_or(0),
            },
            recent_courses: course_responses,
        })
    }
}
