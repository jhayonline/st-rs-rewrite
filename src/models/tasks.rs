use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub use super::_entities::tasks::{ActiveModel, Entity, Model};
pub type Tasks = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTaskParams {
    pub course_id: i32,
    pub title: String,
    pub description: String,
    pub requirements: Option<Vec<String>>,
    pub deadline: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTaskParams {
    pub title: Option<String>,
    pub description: Option<String>,
    pub requirements: Option<Vec<String>>,
    pub deadline: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskResponse {
    pub id: i32,
    pub course_id: i32,
    pub mentor_id: i32,
    pub title: String,
    pub description: String,
    pub requirements: Option<Vec<String>>,
    pub deadline: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for TaskResponse {
    fn from(task: Model) -> Self {
        let requirements = task.requirements.clone().and_then(|json| {
            serde_json::from_value::<Vec<String>>(json).ok()
        });

        Self {
            id: task.id,
            course_id: task.course_id,
            mentor_id: task.mentor_id,
            title: task.title,
            description: task.description,
            requirements,
            deadline: task.deadline,
            status: task.status,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
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
    /// Find task by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find all tasks for a course
    pub async fn find_by_course(db: &DatabaseConnection, course_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::tasks::Column::CourseId.eq(course_id))
            .order_by_desc(super::_entities::tasks::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find active tasks for a course
    pub async fn find_active_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::tasks::Column::CourseId.eq(course_id))
            .filter(super::_entities::tasks::Column::Status.eq("active"))
            .order_by_desc(super::_entities::tasks::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find tasks by mentor
    pub async fn find_by_mentor(db: &DatabaseConnection, mentor_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::tasks::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::tasks::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find tasks with upcoming deadlines
    pub async fn find_upcoming_deadlines(
        db: &DatabaseConnection,
        days_ahead: i64,
    ) -> Result<Vec<Self>, DbErr> {
        let now = chrono::Utc::now();
        let future = now + chrono::Duration::days(days_ahead);

        // Convert to FixedOffset for comparison with database
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();
        let future_fixed: chrono::DateTime<chrono::FixedOffset> = future.into();

        Entity::find()
            .filter(super::_entities::tasks::Column::Status.eq("active"))
            .filter(super::_entities::tasks::Column::Deadline.is_not_null())
            .filter(super::_entities::tasks::Column::Deadline.gt(now_fixed))
            .filter(super::_entities::tasks::Column::Deadline.lt(future_fixed))
            .order_by_asc(super::_entities::tasks::Column::Deadline)
            .all(db)
            .await
    }

    /// Check if deadline has passed
    pub fn is_deadline_passed(&self) -> bool {
        if let Some(deadline) = self.deadline {
            let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
            deadline < now
        } else {
            false
        }
    }

    /// Parse requirements from JSON to Vec<String>
    pub fn parse_requirements(&self) -> Vec<String> {
        self.requirements
            .clone()
            .and_then(|json| serde_json::from_value::<Vec<String>>(json).ok())
            .unwrap_or_default()
    }
}

impl ActiveModel {
    pub async fn create(
        db: &DatabaseConnection,
        params: &CreateTaskParams,
    ) -> Result<Model, DbErr> {
        use crate::models::_entities::courses;
        
        let course = courses::Entity::find_by_id(params.course_id)
            .one(db)
            .await?;
        
        let course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;

        // Convert requirements to JSON
        let requirements_json = params
            .requirements
            .clone()
            .map(|reqs| serde_json::to_value(reqs).unwrap_or(JsonValue::Null));

        let task = ActiveModel {
            course_id: sea_orm::ActiveValue::Set(params.course_id),
            mentor_id: sea_orm::ActiveValue::Set(course.mentor_id),
            title: sea_orm::ActiveValue::Set(params.title.clone()),
            description: sea_orm::ActiveValue::Set(params.description.clone()),
            requirements: sea_orm::ActiveValue::Set(requirements_json),
            deadline: sea_orm::ActiveValue::Set(params.deadline),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        
        Ok(task)
    }

    pub async fn update(
        db: &DatabaseConnection,
        task_id: i32,
        params: &UpdateTaskParams,
    ) -> Result<Model, DbErr> {
        let task = Model::find_by_id(db, task_id).await?;
        let task = task.ok_or(DbErr::RecordNotFound("Task not found".to_string()))?;

        let mut active_model: ActiveModel = task.into();

        if let Some(title) = &params.title {
            active_model.title = sea_orm::ActiveValue::Set(title.clone());
        }
        if let Some(description) = &params.description {
            active_model.description = sea_orm::ActiveValue::Set(description.clone());
        }
        if let Some(requirements) = &params.requirements {
            let json = serde_json::to_value(requirements).unwrap_or(JsonValue::Null);
            active_model.requirements = sea_orm::ActiveValue::Set(Some(json));
        }
        if let Some(deadline) = &params.deadline {
            active_model.deadline = sea_orm::ActiveValue::Set(Some(*deadline));
        }
        if let Some(status) = &params.status {
            active_model.status = sea_orm::ActiveValue::Set(status.clone());
        }

        let updated_task = active_model.update(db).await?;
        Ok(updated_task)
    }

    pub async fn update_status(
        db: &DatabaseConnection,
        task_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        let task = Model::find_by_id(db, task_id).await?;
        let task = task.ok_or(DbErr::RecordNotFound("Task not found".to_string()))?;

        let mut active_model: ActiveModel = task.into();
        active_model.status = sea_orm::ActiveValue::Set(status.to_string());

        let updated_task = active_model.update(db).await?;
        Ok(updated_task)
    }

    /// Soft a task
    pub async fn delete(db: &DatabaseConnection, task_id: i32) -> Result<Model, DbErr> {
        Self::update_status(db, task_id, "archived").await
    }

    /// Hard delete a task
    pub async fn hard_delete(db: &DatabaseConnection, task_id: i32) -> Result<(), DbErr> {
        let task = Model::find_by_id(db, task_id).await?;
        let task = task.ok_or(DbErr::RecordNotFound("Task not found".to_string()))?;

        // Check if there are submissions
        use crate::models::_entities::task_submissions;
        
        let submissions = task_submissions::Entity::find()
            .filter(task_submissions::Column::TaskId.eq(task_id))
            .count(db)
            .await?;
        
        if submissions > 0 {
            return Err(DbErr::RecordNotUpdated);
        }

        let active_model: ActiveModel = task.into();
        active_model.delete(db).await?;
        Ok(())
    }
}

impl Entity {
    /// Find tasks with course information
    pub async fn find_with_course(
        db: &DatabaseConnection,
        task_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::courses::Model)>, DbErr> {
        use crate::models::_entities::courses;
        
        let result = Entity::find()
            .find_also_related(courses::Entity)
            .filter(super::_entities::tasks::Column::Id.eq(task_id))
            .one(db)
            .await?;
        
        match result {
            Some((task, Some(course))) => Ok(Some((task, course))),
            Some((_task, None)) => Err(DbErr::RecordNotFound("Course not found".to_string())),
            None => Ok(None),
        }
    }

    /// Get tasks with submission status for a specific mentee
    pub async fn find_with_submission_status(
        db: &DatabaseConnection,
        course_id: i32,
        mentee_id: i32,
    ) -> Result<Vec<(Model, Option<String>)>, DbErr> {
        use crate::models::_entities::task_submissions;
        
        let tasks = Entity::find()
            .filter(super::_entities::tasks::Column::CourseId.eq(course_id))
            .filter(super::_entities::tasks::Column::Status.eq("active"))
            .order_by_desc(super::_entities::tasks::Column::CreatedAt)
            .all(db)
            .await?;

        let mut result = Vec::new();
        for task in tasks {
            let submission = task_submissions::Entity::find()
                .filter(task_submissions::Column::TaskId.eq(task.id))
                .filter(task_submissions::Column::MenteeId.eq(mentee_id))
                .one(db)
                .await?;

            let status = submission.map(|s| s.status);
            result.push((task, status));
        }

        Ok(result)
    }

    pub async fn count_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::tasks::Column::CourseId.eq(course_id))
            .count(db)
            .await?;
        
        Ok(count.try_into().unwrap_or(0))
    }

    pub async fn get_stats_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<(i64, i64, i64), DbErr> {
        use crate::models::_entities::task_submissions;
        
        let total = Entity::find()
            .filter(super::_entities::tasks::Column::MentorId.eq(mentor_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let active = Entity::find()
            .filter(super::_entities::tasks::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::tasks::Column::Status.eq("active"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let pending_review = task_submissions::Entity::find()
            .inner_join(Entity)
            .filter(super::_entities::tasks::Column::MentorId.eq(mentor_id))
            .filter(task_submissions::Column::Status.eq("pending"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, active, pending_review))
    }
}
