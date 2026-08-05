use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};

pub use super::_entities::courses::{ActiveModel, Entity, Model};
pub type Courses = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCourseParams {
    pub name: String,
    pub duration: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateCourseParams {
    pub name: Option<String>,
    pub duration: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
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
    /// Find course by ID with optional error
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find all courses for a mentor
    pub async fn find_by_mentor(db: &DatabaseConnection, mentor_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::courses::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::courses::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find active courses for a mentor
    pub async fn find_active_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::courses::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::courses::Column::Status.eq("active"))
            .order_by_desc(super::_entities::courses::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find courses by status
    pub async fn find_by_status(db: &DatabaseConnection, status: &str) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::courses::Column::Status.eq(status))
            .order_by_desc(super::_entities::courses::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Check if a course name is unique for a mentor
    ///
    /// Purpose: Ensures a mentor can't have two courses with the same name
    ///
    /// exclude_id: When updating, exclude the current course from the check
    ///
    /// Returns: true if name is available, false if taken
    pub async fn is_name_unique(
        db: &DatabaseConnection,
        mentor_id: i32,
        name: &str,
        exclude_id: Option<i32>,
    ) -> Result<bool, DbErr> {
        let mut query = Entity::find()
            .filter(super::_entities::courses::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::courses::Column::Name.eq(name));
        
        if let Some(id) = exclude_id {
            query = query.filter(super::_entities::courses::Column::Id.ne(id));
        }
        
        let count = query.count(db).await?;
        Ok(count == 0)
    }
}

impl ActiveModel {
    pub async fn create(
        db: &DatabaseConnection,
        mentor_id: i32,
        params: &CreateCourseParams,
    ) -> Result<Model, DbErr> {
        // Check name uniqueness
        if !Model::is_name_unique(db, mentor_id, &params.name, None).await? {
            return Err(DbErr::RecordNotInserted);
        }

        let course = ActiveModel {
            mentor_id: sea_orm::ActiveValue::Set(mentor_id),
            name: sea_orm::ActiveValue::Set(params.name.clone()),
            duration: sea_orm::ActiveValue::Set(params.duration.clone()),
            description: sea_orm::ActiveValue::Set(params.description.clone()),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            enrolled_mentees_count: sea_orm::ActiveValue::Set(Some(0)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        
        Ok(course)
    }

    /// Update an existing course
    pub async fn update(
        db: &DatabaseConnection,
        course_id: i32,
        params: &UpdateCourseParams,
    ) -> Result<Model, DbErr> {
        let course = Model::find_by_id(db, course_id).await?;
        let course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        // Check name uniqueness if name is changing
        if let Some(name) = &params.name {
            if name != &course.name {
                if !Model::is_name_unique(db, course.mentor_id, name, Some(course_id)).await? {
                    return Err(DbErr::RecordNotInserted);
                }
            }
        }

        let mut active_model: ActiveModel = course.into();

        if let Some(name) = &params.name {
            active_model.name = sea_orm::ActiveValue::Set(name.clone());
        }
        if let Some(duration) = &params.duration {
            active_model.duration = sea_orm::ActiveValue::Set(duration.clone());
        }
        if let Some(description) = &params.description {
            active_model.description = sea_orm::ActiveValue::Set(description.clone());
        }
        if let Some(status) = &params.status {
            active_model.status = sea_orm::ActiveValue::Set(status.clone());
        }

        let updated_course = active_model.update(db).await?;
        Ok(updated_course)
    }

    /// Update course status
    pub async fn update_status(
        db: &DatabaseConnection,
        course_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        let course = Model::find_by_id(db, course_id).await?;
        let course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        let mut active_model: ActiveModel = course.into();
        active_model.status = sea_orm::ActiveValue::Set(status.to_string());

        let updated_course = active_model.update(db).await?;
        Ok(updated_course)
    }

    /// Increment enrollment count
    pub async fn increment_enrollment_count(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Model, DbErr> {
        let course = Model::find_by_id(db, course_id).await?;
        let course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        let current_count = course.enrolled_mentees_count.unwrap_or(0);
        
        let mut active_model: ActiveModel = course.into();
        active_model.enrolled_mentees_count = sea_orm::ActiveValue::Set(Some(current_count + 1));

        let updated_course = active_model.update(db).await?;
        Ok(updated_course)
    }

    /// Decrement enrollment count
    pub async fn decrement_enrollment_count(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Model, DbErr> {
        let course = Model::find_by_id(db, course_id).await?;
        let course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        let current_count = course.enrolled_mentees_count.unwrap_or(0);
        
        if current_count == 0 {
            return Err(DbErr::RecordNotUpdated);
        }
        
        let mut active_model: ActiveModel = course.into();
        active_model.enrolled_mentees_count = sea_orm::ActiveValue::Set(Some(current_count - 1));

        let updated_course = active_model.update(db).await?;
        Ok(updated_course)
    }
}

// ============================================================
// Entity Implementation (Custom finders/selectors)
// ============================================================

impl Entity {
    /// Find courses with mentor information
    pub async fn find_with_mentor(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        let result = Entity::find()
            .find_also_related(users::Entity)
            .filter(super::_entities::courses::Column::Id.eq(course_id))
            .one(db)
            .await?;
        
        // Convert Option<(Model, Option<Model>)> to Option<(Model, Model)>
        match result {
            Some((course, Some(user))) => Ok(Some((course, user))),
            Some((_course, None)) => Err(DbErr::RecordNotFound("Mentor not found".to_string())),
            None => Ok(None),
        }
    }

    /// Get all courses with stats (lessons, tasks counts)
    pub async fn find_with_stats(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Option<(Model, i64, i64)>, DbErr> {
        use super::_entities::{lessons, tasks};
        
        let course = Model::find_by_id(db, course_id).await?;
        
        if let Some(course) = course {
            let lesson_count = lessons::Entity::find()
                .filter(lessons::Column::CourseId.eq(course_id))
                .count(db)
                .await?;
            
            let task_count = tasks::Entity::find()
                .filter(tasks::Column::CourseId.eq(course_id))
                .count(db)
                .await?;
            
            // Convert u64 to i64 safely
            let lesson_count_i64 = lesson_count.try_into().unwrap_or(0);
            let task_count_i64 = task_count.try_into().unwrap_or(0);
            
            return Ok(Some((course, lesson_count_i64, task_count_i64)));
        }
        
        Ok(None)
    }
}
