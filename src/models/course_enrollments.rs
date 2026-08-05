use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};

pub use super::_entities::course_enrollments::{ActiveModel, Entity, Model};
pub type CourseEnrollments = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct EnrollMenteeParams {
    pub course_id: i32,
    pub mentee_id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkEnrollParams {
    pub course_id: i32,
    pub mentee_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnrollmentResponse {
    pub id: i32,
    pub course_id: i32,
    pub mentee_id: i32,
    pub enrolled_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub completed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub progress_percentage: Option<i32>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for EnrollmentResponse {
    fn from(enrollment: Model) -> Self {
        Self {
            id: enrollment.id,
            course_id: enrollment.course_id,
            mentee_id: enrollment.mentee_id,
            enrolled_at: enrollment.enrolled_at,
            completed_at: enrollment.completed_at,
            progress_percentage: enrollment.progress_percentage,
            status: enrollment.status,
            created_at: enrollment.created_at,
            updated_at: enrollment.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnrollmentWithDetails {
    pub enrollment: EnrollmentResponse,
    pub course_name: String,
    pub mentor_name: String,
    pub mentee_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CourseProgressStats {
    pub total_mentees: i64,
    pub active_mentees: i64,
    pub completed_mentees: i64,
    pub average_progress: i32,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}


impl Model {
    /// Find enrollment by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find enrollment by course and mentee
    pub async fn find_by_course_and_mentee(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Option<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .filter(super::_entities::course_enrollments::Column::MenteeId.eq(mentee_id))
            .one(db)
            .await
    }

    /// Find all enrollments for a course
    pub async fn find_by_course(db: &DatabaseConnection, course_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await
    }

    /// Find active enrollments for a course
    pub async fn find_active_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .filter(super::_entities::course_enrollments::Column::Status.eq("active"))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await
    }

    /// Find all enrollments for a mentee
    pub async fn find_by_mentee(db: &DatabaseConnection, mentee_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::course_enrollments::Column::MenteeId.eq(mentee_id))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await
    }

    /// Find active enrollments for a mentee
    pub async fn find_active_by_mentee(
        db: &DatabaseConnection,
        mentee_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::course_enrollments::Column::MenteeId.eq(mentee_id))
            .filter(super::_entities::course_enrollments::Column::Status.eq("active"))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await
    }

    /// Find enrollments for a mentor (through courses)
    pub async fn find_by_mentor(db: &DatabaseConnection, mentor_id: i32) -> Result<Vec<Self>, DbErr> {
        use crate::models::_entities::courses;
        
        Entity::find()
            .inner_join(courses::Entity)
            .filter(courses::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await
    }

    /// Check if a mentee is enrolled in a course
    pub async fn is_mentee_enrolled(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<bool, DbErr> {
        let enrollment = Self::find_by_course_and_mentee(db, course_id, mentee_id).await?;
        Ok(enrollment.is_some())
    }

    /// Get enrollment count for a course
    pub async fn get_enrollment_count(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .filter(super::_entities::course_enrollments::Column::Status.eq("active"))
            .count(db)
            .await?;
        
        Ok(count.try_into().unwrap_or(0))
    }

    /// Get progress statistics for a course
    pub async fn get_course_stats(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<CourseProgressStats, DbErr> {
        let total = Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let active = Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .filter(super::_entities::course_enrollments::Column::Status.eq("active"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let completed = Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .filter(super::_entities::course_enrollments::Column::Status.eq("completed"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        // Calculate average progress
        let avg_progress = Entity::find()
            .filter(super::_entities::course_enrollments::Column::CourseId.eq(course_id))
            .filter(super::_entities::course_enrollments::Column::Status.eq("active"))
            .all(db)
            .await?;

        let avg = if !avg_progress.is_empty() {
            let sum: i32 = avg_progress
                .iter()
                .filter_map(|e| e.progress_percentage)
                .sum();
            sum / avg_progress.len() as i32
        } else {
            0
        };

        Ok(CourseProgressStats {
            total_mentees: total,
            active_mentees: active,
            completed_mentees: completed,
            average_progress: avg,
        })
    }
}

impl ActiveModel {
    /// Enroll a mentee in a course
    pub async fn enroll_mentee(
        db: &DatabaseConnection,
        params: &EnrollMenteeParams,
    ) -> Result<Model, DbErr> {
        // Verify course exists
        use crate::models::_entities::courses;
        
        let course = courses::Entity::find_by_id(params.course_id)
            .one(db)
            .await?;
        
        let _course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        // Verify mentee exists
        use crate::models::_entities::users;
        
        let mentee = users::Entity::find_by_id(params.mentee_id)
            .one(db)
            .await?;
        
        let _mentee = mentee.ok_or(DbErr::RecordNotFound("Mentee not found".to_string()))?;

        // Check if already enrolled
        let existing = Model::find_by_course_and_mentee(
            db,
            params.course_id,
            params.mentee_id,
        )
        .await?;

        if existing.is_some() {
            return Err(DbErr::RecordNotInserted);
        }

        // Create enrollment
        let enrollment = ActiveModel {
            course_id: sea_orm::ActiveValue::Set(params.course_id),
            mentee_id: sea_orm::ActiveValue::Set(params.mentee_id),
            enrolled_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into())),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            progress_percentage: sea_orm::ActiveValue::Set(Some(0)),
            ..Default::default()
        }
        .insert(db)
        .await?;

        // Update course enrollment count
        use crate::models::courses::ActiveModel as CourseActiveModel;
        
        let course = courses::Entity::find_by_id(params.course_id)
            .one(db)
            .await?;
        
        if let Some(course) = course {
            // Clone course before moving into ActiveModel
            let mut course_active: CourseActiveModel = course.clone().into();
            let current_count = course.enrolled_mentees_count.unwrap_or(0);
            course_active.enrolled_mentees_count = sea_orm::ActiveValue::Set(Some(current_count + 1));
            course_active.update(db).await?;
        }

        Ok(enrollment)
    }

    /// Bulk enroll mentees in a course
    pub async fn bulk_enroll(
        db: &DatabaseConnection,
        params: &BulkEnrollParams,
    ) -> Result<Vec<Model>, DbErr> {
        let mut enrolled = Vec::new();
        
        for mentee_id in &params.mentee_ids {
            let enroll_params = EnrollMenteeParams {
                course_id: params.course_id,
                mentee_id: *mentee_id,
            };
            
            match Self::enroll_mentee(db, &enroll_params).await {
                Ok(enrollment) => enrolled.push(enrollment),
                Err(DbErr::RecordNotInserted) => {
                    // Already enrolled, skip
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        Ok(enrolled)
    }

    /// Update enrollment status
    pub async fn update_status(
        db: &DatabaseConnection,
        enrollment_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        if !["active", "completed", "dropped"].contains(&status) {
            return Err(DbErr::RecordNotUpdated);
        }

        let enrollment = Model::find_by_id(db, enrollment_id).await?;
        let enrollment = enrollment.ok_or(DbErr::RecordNotFound("Enrollment not found".to_string()))?;

        let mut active_model: ActiveModel = enrollment.into();
        active_model.status = sea_orm::ActiveValue::Set(status.to_string());

        // If completing, set completed_at
        if status == "completed" {
            active_model.completed_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Update progress percentage
    pub async fn update_progress(
        db: &DatabaseConnection,
        enrollment_id: i32,
        progress: i32,
    ) -> Result<Model, DbErr> {
        if !(0..=100).contains(&progress) {
            return Err(DbErr::RecordNotUpdated);
        }

        let enrollment = Model::find_by_id(db, enrollment_id).await?;
        let enrollment = enrollment.ok_or(DbErr::RecordNotFound("Enrollment not found".to_string()))?;

        // Clone enrollment to check status after moving
        let status = enrollment.status.clone();
        
        let mut active_model: ActiveModel = enrollment.into();
        active_model.progress_percentage = sea_orm::ActiveValue::Set(Some(progress));

        // Auto-complete if progress reaches 100
        if progress == 100 && status != "completed" {
            active_model.status = sea_orm::ActiveValue::Set("completed".to_string());
            active_model.completed_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Complete a course for a mentee
    pub async fn complete_course(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Model, DbErr> {
        let enrollment = Model::find_by_course_and_mentee(db, course_id, mentee_id).await?;
        let enrollment = enrollment.ok_or(DbErr::RecordNotFound("Enrollment not found".to_string()))?;

        Self::update_status(db, enrollment.id, "completed").await
    }

    /// Drop a course for a mentee
    pub async fn drop_course(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Model, DbErr> {
        let enrollment = Model::find_by_course_and_mentee(db, course_id, mentee_id).await?;
        let enrollment = enrollment.ok_or(DbErr::RecordNotFound("Enrollment not found".to_string()))?;

        Self::update_status(db, enrollment.id, "dropped").await
    }

    /// Delete an enrollment (hard delete)
    pub async fn hard_delete(db: &DatabaseConnection, enrollment_id: i32) -> Result<(), DbErr> {
        let enrollment = Model::find_by_id(db, enrollment_id).await?;
        let enrollment = enrollment.ok_or(DbErr::RecordNotFound("Enrollment not found".to_string()))?;

        // Store course_id before moving enrollment
        let course_id = enrollment.course_id;

        // Update course enrollment count
        use crate::models::_entities::courses;
        
        let course = courses::Entity::find_by_id(course_id)
            .one(db)
            .await?;
        
        if let Some(course) = course {
            use crate::models::courses::ActiveModel as CourseActiveModel;
            // Clone course before moving into ActiveModel
            let mut course_active: CourseActiveModel = course.clone().into();
            let current_count = course.enrolled_mentees_count.unwrap_or(0);
            if current_count > 0 {
                course_active.enrolled_mentees_count = sea_orm::ActiveValue::Set(Some(current_count - 1));
                course_active.update(db).await?;
            }
        }

        let active_model: ActiveModel = enrollment.into();
        active_model.delete(db).await?;
        Ok(())
    }

    /// Calculate and update progress for a mentee in a course
    pub async fn calculate_progress(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<i32, DbErr> {
        use crate::models::_entities::{lessons, lesson_progress};
        
        // Get total active lessons in course
        let total_lessons = lessons::Entity::find()
            .filter(lessons::Column::CourseId.eq(course_id))
            .filter(lessons::Column::Status.eq("active"))
            .count(db)
            .await?;

        if total_lessons == 0 {
            return Ok(0);
        }

        // Get completed lessons for this mentee in this course
        let completed_lessons = lesson_progress::Entity::find()
            .inner_join(lessons::Entity)
            .filter(lessons::Column::CourseId.eq(course_id))
            .filter(lesson_progress::Column::MenteeId.eq(mentee_id))
            .filter(lesson_progress::Column::Completed.eq(true))
            .count(db)
            .await?;

        let total = total_lessons.try_into().unwrap_or(0);
        let completed = completed_lessons.try_into().unwrap_or(0);
        
        let progress = if total > 0 {
            ((completed as f32 / total as f32) * 100.0) as i32
        } else {
            0
        };

        // Update enrollment progress
        let enrollment = Model::find_by_course_and_mentee(db, course_id, mentee_id).await?;
        
        if let Some(enrollment) = enrollment {
            Self::update_progress(db, enrollment.id, progress).await?;
        }

        Ok(progress)
    }
}

impl Entity {
    /// Get enrollment with course and mentee details
    pub async fn find_with_details(
        db: &DatabaseConnection,
        enrollment_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::courses::Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::{courses, users};
        
        let result = Entity::find()
            .find_also_related(courses::Entity)
            .find_also_related(users::Entity)
            .filter(super::_entities::course_enrollments::Column::Id.eq(enrollment_id))
            .one(db)
            .await?;

        match result {
            Some((enrollment, Some(course), Some(mentee))) => Ok(Some((enrollment, course, mentee))),
            _ => Ok(None),
        }
    }

    /// Get all enrollments for a mentor with course info
    pub async fn find_by_mentor_with_courses(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::courses::Model)>, DbErr> {
        use crate::models::_entities::courses;
        
        let result = Entity::find()
            .find_also_related(courses::Entity)
            .filter(courses::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await?;

        let mut enrollments = Vec::new();
        for (enrollment, course) in result {
            if let Some(course) = course {
                enrollments.push((enrollment, course));
            }
        }
        
        Ok(enrollments)
    }

    /// Get all enrollments for a mentee with course info
    pub async fn find_by_mentee_with_courses(
        db: &DatabaseConnection,
        mentee_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::courses::Model)>, DbErr> {
        use crate::models::_entities::courses;
        
        let result = Entity::find()
            .find_also_related(courses::Entity)
            .filter(super::_entities::course_enrollments::Column::MenteeId.eq(mentee_id))
            .order_by_desc(super::_entities::course_enrollments::Column::EnrolledAt)
            .all(db)
            .await?;

        let mut enrollments = Vec::new();
        for (enrollment, course) in result {
            if let Some(course) = course {
                enrollments.push((enrollment, course));
            }
        }
        
        Ok(enrollments)
    }
}
