use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

pub use super::_entities::lesson_progress::{ActiveModel, Entity, Model};
pub type LessonProgress = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProgressParams {
    pub lesson_id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProgressResponse {
    pub id: i32,
    pub lesson_id: i32,
    pub mentee_id: i32,
    pub completed: Option<bool>,
    pub completed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for ProgressResponse {
    fn from(progress: Model) -> Self {
        Self {
            id: progress.id,
            lesson_id: progress.lesson_id,
            mentee_id: progress.mentee_id,
            completed: progress.completed,
            completed_at: progress.completed_at,
            created_at: progress.created_at,
            updated_at: progress.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LessonProgressSummary {
    pub lesson_id: i32,
    pub completed: bool,
    pub completed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CourseProgressSummary {
    pub course_id: i32,
    pub total_lessons: i64,
    pub completed_lessons: i64,
    pub progress_percentage: i32,
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
    /// Find progress by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find progress by lesson and mentee
    pub async fn find_by_lesson_and_mentee(
        db: &DatabaseConnection,
        lesson_id: i32,
        mentee_id: i32,
    ) -> Result<Option<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::lesson_progress::Column::LessonId.eq(lesson_id))
            .filter(super::_entities::lesson_progress::Column::MenteeId.eq(mentee_id))
            .one(db)
            .await
    }

    /// Find all progress records for a mentee
    pub async fn find_by_mentee(db: &DatabaseConnection, mentee_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::lesson_progress::Column::MenteeId.eq(mentee_id))
            .order_by_desc(super::_entities::lesson_progress::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find all progress records for a lesson
    pub async fn find_by_lesson(db: &DatabaseConnection, lesson_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::lesson_progress::Column::LessonId.eq(lesson_id))
            .all(db)
            .await
    }

    /// Find completed lessons for a mentee in a course
    pub async fn find_completed_by_course(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        use crate::models::_entities::lessons;
        
        Entity::find()
            .inner_join(lessons::Entity)
            .filter(lessons::Column::CourseId.eq(course_id))
            .filter(super::_entities::lesson_progress::Column::MenteeId.eq(mentee_id))
            .filter(super::_entities::lesson_progress::Column::Completed.eq(true))
            .all(db)
            .await
    }

    /// Get progress summary for a course
    pub async fn get_course_summary(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<CourseProgressSummary, DbErr> {
        use crate::models::_entities::lessons;
        
        // Get total lessons in course
        let total_lessons = lessons::Entity::find()
            .filter(lessons::Column::CourseId.eq(course_id))
            .filter(lessons::Column::Status.eq("active"))
            .count(db)
            .await?;

        // Get completed lessons
        let completed_lessons = Entity::find()
            .inner_join(lessons::Entity)
            .filter(lessons::Column::CourseId.eq(course_id))
            .filter(super::_entities::lesson_progress::Column::MenteeId.eq(mentee_id))
            .filter(super::_entities::lesson_progress::Column::Completed.eq(true))
            .count(db)
            .await?;

        let total = total_lessons.try_into().unwrap_or(0);
        let completed = completed_lessons.try_into().unwrap_or(0);
        
        let percentage = if total > 0 {
            ((completed as f32 / total as f32) * 100.0) as i32
        } else {
            0
        };

        Ok(CourseProgressSummary {
            course_id,
            total_lessons: total,
            completed_lessons: completed,
            progress_percentage: percentage,
        })
    }

    /// Check if a mentee has completed a lesson
    pub async fn is_lesson_completed(
        db: &DatabaseConnection,
        lesson_id: i32,
        mentee_id: i32,
    ) -> Result<bool, DbErr> {
        let progress = Self::find_by_lesson_and_mentee(db, lesson_id, mentee_id).await?;
        
        match progress {
            Some(p) => Ok(p.completed.unwrap_or(false)),
            None => Ok(false),
        }
    }

    /// Get completion count for a lesson
    pub async fn get_completion_count(
        db: &DatabaseConnection,
        lesson_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::lesson_progress::Column::LessonId.eq(lesson_id))
            .filter(super::_entities::lesson_progress::Column::Completed.eq(true))
            .count(db)
            .await?;
        
        Ok(count.try_into().unwrap_or(0))
    }
}

impl ActiveModel {
    /// Mark a lesson as complete for a mentee
    pub async fn complete_lesson(
        db: &DatabaseConnection,
        lesson_id: i32,
        mentee_id: i32,
    ) -> Result<Model, DbErr> {
        use crate::models::_entities::lessons;
        
        let lesson = lessons::Entity::find_by_id(lesson_id)
            .one(db)
            .await?;
        
        let _lesson = lesson.ok_or(DbErr::RecordNotFound("Lesson not found".to_string()))?;

        let existing = Model::find_by_lesson_and_mentee(db, lesson_id, mentee_id).await?;

        if let Some(existing) = existing {
            // Update existing record
            let mut active_model: ActiveModel = existing.into();
            active_model.completed = sea_orm::ActiveValue::Set(Some(true));
            active_model.completed_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
            
            let updated = active_model.update(db).await?;
            Ok(updated)
        } else {
            // Create new progress record
            let progress = ActiveModel {
                lesson_id: sea_orm::ActiveValue::Set(lesson_id),
                mentee_id: sea_orm::ActiveValue::Set(mentee_id),
                completed: sea_orm::ActiveValue::Set(Some(true)),
                completed_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into())),
                ..Default::default()
            }
            .insert(db)
            .await?;
            
            Ok(progress)
        }
    }

    /// Mark a lesson as incomplete for a mentee
    pub async fn uncomplete_lesson(
        db: &DatabaseConnection,
        lesson_id: i32,
        mentee_id: i32,
    ) -> Result<Model, DbErr> {
        let existing = Model::find_by_lesson_and_mentee(db, lesson_id, mentee_id).await?;

        if let Some(existing) = existing {
            let mut active_model: ActiveModel = existing.into();
            active_model.completed = sea_orm::ActiveValue::Set(Some(false));
            active_model.completed_at = sea_orm::ActiveValue::Set(None);
            
            let updated = active_model.update(db).await?;
            Ok(updated)
        } else {
            // Create a new record with completed = false
            let progress = ActiveModel {
                lesson_id: sea_orm::ActiveValue::Set(lesson_id),
                mentee_id: sea_orm::ActiveValue::Set(mentee_id),
                completed: sea_orm::ActiveValue::Set(Some(false)),
                completed_at: sea_orm::ActiveValue::Set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
            
            Ok(progress)
        }
    }

    /// Bulk complete lessons for a mentee
    pub async fn bulk_complete_lessons(
        db: &DatabaseConnection,
        lesson_ids: Vec<i32>,
        mentee_id: i32,
    ) -> Result<Vec<Model>, DbErr> {
        let mut results = Vec::new();
        
        for lesson_id in lesson_ids {
            let progress = Self::complete_lesson(db, lesson_id, mentee_id).await?;
            results.push(progress);
        }
        
        Ok(results)
    }

    /// Delete progress record for resetting
    pub async fn delete_progress(
        db: &DatabaseConnection,
        lesson_id: i32,
        mentee_id: i32,
    ) -> Result<(), DbErr> {
        let existing = Model::find_by_lesson_and_mentee(db, lesson_id, mentee_id).await?;

        if let Some(existing) = existing {
            let active_model: ActiveModel = existing.into();
            active_model.delete(db).await?;
        }
        
        Ok(())
    }
}


impl Entity {
    /// Get progress with lesson and course information
    pub async fn find_with_lesson(
        db: &DatabaseConnection,
        progress_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::lessons::Model)>, DbErr> {
        use crate::models::_entities::lessons;
        
        let result = Entity::find()
            .find_also_related(lessons::Entity)
            .filter(super::_entities::lesson_progress::Column::Id.eq(progress_id))
            .one(db)
            .await?;
        
        match result {
            Some((progress, Some(lesson))) => Ok(Some((progress, lesson))),
            Some((_progress, None)) => Err(DbErr::RecordNotFound("Lesson not found".to_string())),
            None => Ok(None),
        }
    }

    /// Get all lesson progress for a course with lesson details
    pub async fn get_course_progress(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::lessons::Model)>, DbErr> {
        use crate::models::_entities::lessons;
        
        let progress = Entity::find()
            .inner_join(lessons::Entity)
            .filter(lessons::Column::CourseId.eq(course_id))
            .filter(super::_entities::lesson_progress::Column::MenteeId.eq(mentee_id))
            .all(db)
            .await?;

        let mut results = Vec::new();
        for p in progress {
            let lesson = lessons::Entity::find_by_id(p.lesson_id)
                .one(db)
                .await?;
            
            if let Some(lesson) = lesson {
                results.push((p, lesson));
            }
        }
        
        Ok(results)
    }

    /// Get all mentees who have completed a lesson
    pub async fn get_mentees_by_lesson(
        db: &DatabaseConnection,
        lesson_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        let progress = Entity::find()
            .find_also_related(users::Entity)
            .filter(super::_entities::lesson_progress::Column::LessonId.eq(lesson_id))
            .filter(super::_entities::lesson_progress::Column::Completed.eq(true))
            .all(db)
            .await?;

        let mut results = Vec::new();
        for (progress, user) in progress {
            if let Some(user) = user {
                results.push((progress, user));
            }
        }
        
        Ok(results)
    }

    /// Get progress statistics for a mentor (across all their courses)
    pub async fn get_stats_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<(i64, i64), DbErr> {
        use crate::models::_entities::{lessons, courses};
        
        // Get all lessons for this mentor's courses
        let total_lessons = lessons::Entity::find()
            .inner_join(courses::Entity)
            .filter(courses::Column::MentorId.eq(mentor_id))
            .filter(lessons::Column::Status.eq("active"))
            .count(db)
            .await?;

        // Get all lesson IDs for this mentor's courses
        let lesson_ids: Vec<i32> = lessons::Entity::find()
            .inner_join(courses::Entity)
            .filter(courses::Column::MentorId.eq(mentor_id))
            .filter(lessons::Column::Status.eq("active"))
            .select_only()
            .column(lessons::Column::Id)
            .into_tuple()
            .all(db)
            .await?;

        if lesson_ids.is_empty() {
            return Ok((0, 0));
        }

        // Count completed progress for these lessons
        let completed = Entity::find()
            .filter(super::_entities::lesson_progress::Column::LessonId.is_in(lesson_ids))
            .filter(super::_entities::lesson_progress::Column::Completed.eq(true))
            .count(db)
            .await?;

        Ok((
            total_lessons.try_into().unwrap_or(0),
            completed.try_into().unwrap_or(0),
        ))
    }
}
