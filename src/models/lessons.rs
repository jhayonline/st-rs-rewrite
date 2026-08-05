use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, TransactionTrait};  // Add TransactionTrait here!
use serde::{Deserialize, Serialize};

pub use super::_entities::lessons::{ActiveModel, Entity, Model};
pub type Lessons = Entity;

// ============================================================
// DTOs
// ============================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateLessonParams {
    pub course_id: i32,
    pub title: String,
    pub description: String,
    pub link: String,
    pub order_index: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateLessonParams {
    pub title: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
    pub order_index: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReorderLessonsParams {
    pub lesson_ids: Vec<i32>,
}

// ============================================================
// ActiveModelBehavior
// ============================================================

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

// ============================================================
// Model Implementation (Read operations)
// ============================================================

impl Model {
    /// Find lesson by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find all lessons for a course, ordered by order_index
    pub async fn find_by_course(db: &DatabaseConnection, course_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::lessons::Column::CourseId.eq(course_id))
            .order_by_asc(super::_entities::lessons::Column::OrderIndex)
            .all(db)
            .await
    }

    /// Find active lessons for a course
    pub async fn find_active_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::lessons::Column::CourseId.eq(course_id))
            .filter(super::_entities::lessons::Column::Status.eq("active"))
            .order_by_asc(super::_entities::lessons::Column::OrderIndex)
            .all(db)
            .await
    }

    /// Find lessons by mentor
    pub async fn find_by_mentor(db: &DatabaseConnection, mentor_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::lessons::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::lessons::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Get the next order index for a course
    pub async fn get_next_order_index(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<i32, DbErr> {
        let max_order = Entity::find()
            .filter(super::_entities::lessons::Column::CourseId.eq(course_id))
            .order_by_desc(super::_entities::lessons::Column::OrderIndex)
            .one(db)
            .await?;

        Ok(max_order.map(|l| l.order_index + 1).unwrap_or(0))
    }
}

// ============================================================
// ActiveModel Implementation (Write operations)
// ============================================================

impl ActiveModel {
    /// Create a new lesson
    pub async fn create(
        db: &DatabaseConnection,
        params: &CreateLessonParams,
    ) -> Result<Model, DbErr> {
        // Verify course exists and get mentor_id
        use crate::models::_entities::courses;
        
        let course = courses::Entity::find_by_id(params.course_id)
            .one(db)
            .await?;
        
        let course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        // Get next order index if not provided
        let order_index = params.order_index.unwrap_or(0);

        let lesson = ActiveModel {
            course_id: sea_orm::ActiveValue::Set(params.course_id),
            mentor_id: sea_orm::ActiveValue::Set(course.mentor_id),
            title: sea_orm::ActiveValue::Set(params.title.clone()),
            description: sea_orm::ActiveValue::Set(params.description.clone()),
            link: sea_orm::ActiveValue::Set(params.link.clone()),
            order_index: sea_orm::ActiveValue::Set(order_index),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        
        Ok(lesson)
    }

    /// Update an existing lesson
    pub async fn update(
        db: &DatabaseConnection,
        lesson_id: i32,
        params: &UpdateLessonParams,
    ) -> Result<Model, DbErr> {
        let lesson = Model::find_by_id(db, lesson_id).await?;
        let lesson = lesson.ok_or(DbErr::RecordNotFound("Lesson not found".to_string()))?;

        let mut active_model: ActiveModel = lesson.into();

        if let Some(title) = &params.title {
            active_model.title = sea_orm::ActiveValue::Set(title.clone());
        }
        if let Some(description) = &params.description {
            active_model.description = sea_orm::ActiveValue::Set(description.clone());
        }
        if let Some(link) = &params.link {
            active_model.link = sea_orm::ActiveValue::Set(link.clone());
        }
        if let Some(order_index) = &params.order_index {
            active_model.order_index = sea_orm::ActiveValue::Set(*order_index);
        }
        if let Some(status) = &params.status {
            active_model.status = sea_orm::ActiveValue::Set(status.clone());
        }

        let updated_lesson = active_model.update(db).await?;
        Ok(updated_lesson)
    }

    /// Update lesson status
    pub async fn update_status(
        db: &DatabaseConnection,
        lesson_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        let lesson = Model::find_by_id(db, lesson_id).await?;
        let lesson = lesson.ok_or(DbErr::RecordNotFound("Lesson not found".to_string()))?;

        let mut active_model: ActiveModel = lesson.into();
        active_model.status = sea_orm::ActiveValue::Set(status.to_string());

        let updated_lesson = active_model.update(db).await?;
        Ok(updated_lesson)
    }

    /// Delete a lesson (soft delete - set status to archived)
    pub async fn delete(db: &DatabaseConnection, lesson_id: i32) -> Result<Model, DbErr> {
        Self::update_status(db, lesson_id, "archived").await
    }

    /// Hard delete a lesson (permanent)
    pub async fn hard_delete(db: &DatabaseConnection, lesson_id: i32) -> Result<(), DbErr> {
        let lesson = Model::find_by_id(db, lesson_id).await?;
        let lesson = lesson.ok_or(DbErr::RecordNotFound("Lesson not found".to_string()))?;

        let active_model: ActiveModel = lesson.into();
        active_model.delete(db).await?;
        Ok(())
    }

    /// Reorder lessons within a course
    pub async fn reorder(
        db: &DatabaseConnection,
        course_id: i32,
        lesson_ids: Vec<i32>,
    ) -> Result<Vec<Model>, DbErr> {
        // Start a transaction
        let txn = db.begin().await?;

        // Update each lesson's order_index
        for (index, lesson_id) in lesson_ids.iter().enumerate() {
            // Use Entity directly instead of Model::find_by_id
            let lesson = Entity::find_by_id(*lesson_id)
                .one(&txn)
                .await?;
            
            let lesson = lesson.ok_or(DbErr::RecordNotFound("Lesson not found".to_string()))?;

            // Verify lesson belongs to this course
            if lesson.course_id != course_id {
                return Err(DbErr::RecordNotUpdated);
            }

            let mut active_model: ActiveModel = lesson.into();
            active_model.order_index = sea_orm::ActiveValue::Set(index as i32);
            active_model.update(&txn).await?;
        }

        txn.commit().await?;

        // Return the reordered lessons
        Model::find_by_course(db, course_id).await
    }
}

// ============================================================
// Entity Implementation (Custom finders/selectors)
// ============================================================

impl Entity {
    /// Find lessons with course information
    pub async fn find_with_course(
        db: &DatabaseConnection,
        lesson_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::courses::Model)>, DbErr> {
        use crate::models::_entities::courses;
        
        let result = Entity::find()
            .find_also_related(courses::Entity)
            .filter(super::_entities::lessons::Column::Id.eq(lesson_id))
            .one(db)
            .await?;
        
        match result {
            Some((lesson, Some(course))) => Ok(Some((lesson, course))),
            Some((_lesson, None)) => Err(DbErr::RecordNotFound("Course not found".to_string())),
            None => Ok(None),
        }
    }

    /// Get all lessons for a course with progress for a specific mentee
    pub async fn find_with_progress(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Vec<(Model, Option<bool>)>, DbErr> {
        use crate::models::_entities::lesson_progress;
        
        let lessons = Entity::find()
            .filter(super::_entities::lessons::Column::CourseId.eq(course_id))
            .order_by_asc(super::_entities::lessons::Column::OrderIndex)
            .all(db)
            .await?;

        let mut result = Vec::new();
        for lesson in lessons {
            let progress = lesson_progress::Entity::find()
                .filter(lesson_progress::Column::LessonId.eq(lesson.id))
                .filter(lesson_progress::Column::MenteeId.eq(mentee_id))
                .one(db)
                .await?;

            let completed = progress.map(|p| p.completed.unwrap_or(false));
            result.push((lesson, completed));
        }

        Ok(result)
    }

    /// Count lessons by course
    pub async fn count_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::lessons::Column::CourseId.eq(course_id))
            .count(db)
            .await?;
        
        // Convert u64 to i64 safely
        Ok(count.try_into().unwrap_or(0))
    }
}
