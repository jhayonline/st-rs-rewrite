use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};

pub use super::_entities::task_submissions::{ActiveModel, Entity, Model};
pub type TaskSubmissions = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSubmissionParams {
    pub task_id: i32,
    pub submission_link: String,
    pub submission_notes: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateSubmissionParams {
    pub submission_link: Option<String>,
    pub submission_notes: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewSubmissionParams {
    pub status: String, // "approved" or "rejected"
    pub mentor_feedback: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubmissionResponse {
    pub id: i32,
    pub task_id: i32,
    pub mentee_id: i32,
    pub submission_link: Option<String>,
    pub submission_notes: Option<String>,
    pub submitted_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub status: String,
    pub mentor_feedback: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for SubmissionResponse {
    fn from(submission: Model) -> Self {
        Self {
            id: submission.id,
            task_id: submission.task_id,
            mentee_id: submission.mentee_id,
            submission_link: submission.submission_link,
            submission_notes: submission.submission_notes,
            submitted_at: submission.submitted_at,
            status: submission.status,
            mentor_feedback: submission.mentor_feedback,
            reviewed_at: submission.reviewed_at,
            created_at: submission.created_at,
            updated_at: submission.updated_at,
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
    /// Find submission by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find all submissions for a task
    pub async fn find_by_task(db: &DatabaseConnection, task_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::task_submissions::Column::TaskId.eq(task_id))
            .order_by_desc(super::_entities::task_submissions::Column::SubmittedAt)
            .all(db)
            .await
    }

    /// Find all submissions by a mentee
    pub async fn find_by_mentee(db: &DatabaseConnection, mentee_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::task_submissions::Column::MenteeId.eq(mentee_id))
            .order_by_desc(super::_entities::task_submissions::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find submissions for a mentor through tasks
    pub async fn find_by_mentor(db: &DatabaseConnection, mentor_id: i32) -> Result<Vec<Self>, DbErr> {
        use crate::models::_entities::tasks;
        
        Entity::find()
            .inner_join(tasks::Entity)
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::task_submissions::Column::SubmittedAt)
            .all(db)
            .await
    }

    /// Find pending submissions for a mentor
    pub async fn find_pending_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        use crate::models::_entities::tasks;
        
        Entity::find()
            .inner_join(tasks::Entity)
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::task_submissions::Column::Status.eq("pending"))
            .order_by_desc(super::_entities::task_submissions::Column::SubmittedAt)
            .all(db)
            .await
    }

    /// Find submission by task and mentee
    pub async fn find_by_task_and_mentee(
        db: &DatabaseConnection,
        task_id: i32,
        mentee_id: i32,
    ) -> Result<Option<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::task_submissions::Column::TaskId.eq(task_id))
            .filter(super::_entities::task_submissions::Column::MenteeId.eq(mentee_id))
            .one(db)
            .await
    }

    pub fn is_pending(&self) -> bool { self.status == "pending" }

    pub fn is_approved(&self) -> bool { self.status == "approved" }

    pub fn is_rejected(&self) -> bool { self.status == "rejected" }

    pub fn needs_review(&self) -> bool { self.status == "pending" || self.status == "submitted" }
}


impl ActiveModel {
    pub async fn create(
        db: &DatabaseConnection,
        mentee_id: i32,
        params: &CreateSubmissionParams,
    ) -> Result<Model, DbErr> {
        use crate::models::_entities::tasks;
        
        let task = tasks::Entity::find_by_id(params.task_id)
            .one(db)
            .await?;
        
        let _task = task.ok_or(DbErr::RecordNotFound("Task not found".to_string()))?;

        let existing = Model::find_by_task_and_mentee(db, params.task_id, mentee_id).await?;
        
        if let Some(existing) = existing {
            // If rejected, allow resubmission
            if existing.status == "rejected" {
                // Update existing submission
                let mut active_model: ActiveModel = existing.into();
                active_model.submission_link = sea_orm::ActiveValue::Set(Some(params.submission_link.clone()));
                active_model.submission_notes = sea_orm::ActiveValue::Set(params.submission_notes.clone());
                active_model.status = sea_orm::ActiveValue::Set("pending".to_string());
                active_model.submitted_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
                active_model.mentor_feedback = sea_orm::ActiveValue::Set(None);
                active_model.reviewed_at = sea_orm::ActiveValue::Set(None);
                
                let updated = active_model.update(db).await?;
                return Ok(updated);
            } else {
                return Err(DbErr::RecordNotInserted);
            }
        }

        // Create new submission
        let submission = ActiveModel {
            task_id: sea_orm::ActiveValue::Set(params.task_id),
            mentee_id: sea_orm::ActiveValue::Set(mentee_id),
            submission_link: sea_orm::ActiveValue::Set(Some(params.submission_link.clone())),
            submission_notes: sea_orm::ActiveValue::Set(params.submission_notes.clone()),
            submitted_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into())),
            status: sea_orm::ActiveValue::Set("pending".to_string()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        
        Ok(submission)
    }

    /// Update a submission (mentee editing)
    pub async fn update(
        db: &DatabaseConnection,
        submission_id: i32,
        params: &UpdateSubmissionParams,
    ) -> Result<Model, DbErr> {
        let submission = Model::find_by_id(db, submission_id).await?;
        let submission = submission.ok_or(DbErr::RecordNotFound("Submission not found".to_string()))?;

        // Only allow updates if pending
        if !submission.is_pending() {
            return Err(DbErr::RecordNotUpdated);
        }

        let mut active_model: ActiveModel = submission.into();

        if let Some(link) = &params.submission_link {
            active_model.submission_link = sea_orm::ActiveValue::Set(Some(link.clone()));
        }
        if let Some(notes) = &params.submission_notes {
            active_model.submission_notes = sea_orm::ActiveValue::Set(Some(notes.clone()));
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Review a submission (mentor)
    pub async fn review(
        db: &DatabaseConnection,
        submission_id: i32,
        params: &ReviewSubmissionParams,
        mentor_id: i32,
    ) -> Result<Model, DbErr> {
        // Validate status
        if !["approved", "rejected"].contains(&params.status.as_str()) {
            return Err(DbErr::RecordNotUpdated);
        }

        // Verify mentor owns the task
        use crate::models::_entities::tasks;
        
        let submission = Entity::find()
            .find_also_related(tasks::Entity)
            .filter(super::_entities::task_submissions::Column::Id.eq(submission_id))
            .one(db)
            .await?;

        match submission {
            Some((submission, Some(task))) => {
                if task.mentor_id != mentor_id {
                    return Err(DbErr::RecordNotUpdated);
                }

                // Rejected submissions require feedback
                if params.status == "rejected" && params.mentor_feedback.is_none() {
                    return Err(DbErr::RecordNotUpdated);
                }

                let mut active_model: ActiveModel = submission.into();
                active_model.status = sea_orm::ActiveValue::Set(params.status.clone());
                active_model.mentor_feedback = sea_orm::ActiveValue::Set(params.mentor_feedback.clone());
                active_model.reviewed_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));

                let updated = active_model.update(db).await?;
                Ok(updated)
            }
            Some((_submission, None)) => Err(DbErr::RecordNotFound("Task not found".to_string())),
            None => Err(DbErr::RecordNotFound("Submission not found".to_string())),
        }
    }

    /// Update submission status (admin/mentor override)
    pub async fn update_status(
        db: &DatabaseConnection,
        submission_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        if !["pending", "submitted", "approved", "rejected"].contains(&status) {
            return Err(DbErr::RecordNotUpdated);
        }

        let submission = Model::find_by_id(db, submission_id).await?;
        let submission = submission.ok_or(DbErr::RecordNotFound("Submission not found".to_string()))?;

        let mut active_model: ActiveModel = submission.into();
        active_model.status = sea_orm::ActiveValue::Set(status.to_string());

        let updated = active_model.update(db).await?;
        Ok(updated)
    }
}

impl Entity {
    /// Find submissions with task and mentee information
    pub async fn find_with_details(
        db: &DatabaseConnection,
        submission_id: i32,
    ) -> Result<
        Option<(
            Model,
            crate::models::_entities::tasks::Model,
            crate::models::_entities::users::Model,
        )>,
        DbErr,
    > {
        use crate::models::_entities::{tasks, users};
        
        let result = Entity::find()
            .find_also_related(tasks::Entity)
            .find_also_related(users::Entity)
            .filter(super::_entities::task_submissions::Column::Id.eq(submission_id))
            .one(db)
            .await?;

        match result {
            Some((submission, Some(task), Some(mentee))) => Ok(Some((submission, task, mentee))),
            _ => Ok(None),
        }
    }

    /// Get submission statistics for a task
    pub async fn get_stats_by_task(
        db: &DatabaseConnection,
        task_id: i32,
    ) -> Result<(i64, i64, i64, i64), DbErr> {
        let total = Entity::find()
            .filter(super::_entities::task_submissions::Column::TaskId.eq(task_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let pending = Entity::find()
            .filter(super::_entities::task_submissions::Column::TaskId.eq(task_id))
            .filter(super::_entities::task_submissions::Column::Status.eq("pending"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let approved = Entity::find()
            .filter(super::_entities::task_submissions::Column::TaskId.eq(task_id))
            .filter(super::_entities::task_submissions::Column::Status.eq("approved"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let rejected = Entity::find()
            .filter(super::_entities::task_submissions::Column::TaskId.eq(task_id))
            .filter(super::_entities::task_submissions::Column::Status.eq("rejected"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, pending, approved, rejected))
    }

    /// Get submission statistics for a mentor
    pub async fn get_stats_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<(i64, i64, i64), DbErr> {
        use crate::models::_entities::tasks;
        
        let total = Entity::find()
            .inner_join(tasks::Entity)
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let pending = Entity::find()
            .inner_join(tasks::Entity)
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::task_submissions::Column::Status.eq("pending"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let reviewed = Entity::find()
            .inner_join(tasks::Entity)
            .filter(tasks::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::task_submissions::Column::Status.is_in(vec!["approved", "rejected"]))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, pending, reviewed))
    }
}
