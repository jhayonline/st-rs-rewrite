use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use sea_orm::Condition;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub use super::_entities::email_queue::{ActiveModel, Entity, Model};
pub type EmailQueue = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct QueueEmailParams {
    pub recipient: String,
    pub recipient_name: Option<String>,
    pub email_type: String,
    pub subject: String,
    pub html_content: String,
    pub text_content: Option<String>,
    pub metadata: Option<JsonValue>,
    pub max_retries: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateEmailParams {
    pub status: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub next_retry_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub sent_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EmailQueueResponse {
    pub id: i32,
    pub recipient: String,
    pub recipient_name: Option<String>,
    pub email_type: String,
    pub subject: String,
    pub html_content: String,
    pub text_content: Option<String>,
    pub metadata: Option<JsonValue>,
    pub status: String,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub next_retry_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub sent_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for EmailQueueResponse {
    fn from(email: Model) -> Self {
        Self {
            id: email.id,
            recipient: email.recipient,
            recipient_name: email.recipient_name,
            email_type: email.email_type,
            subject: email.subject,
            html_content: email.html_content,
            text_content: email.text_content,
            metadata: email.metadata,
            status: email.status,
            error_message: email.error_message,
            retry_count: email.retry_count.unwrap_or(0),
            max_retries: email.max_retries.unwrap_or(3),
            next_retry_at: email.next_retry_at,
            sent_at: email.sent_at,
            created_at: email.created_at,
            updated_at: email.updated_at,
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
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    pub async fn find_pending(db: &DatabaseConnection) -> Result<Vec<Self>, DbErr> {
        let now = chrono::Utc::now();
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();

        Entity::find()
            .filter(super::_entities::email_queue::Column::Status.eq("pending"))
            .filter(
                Condition::any()
                    .add(super::_entities::email_queue::Column::NextRetryAt.is_null())
                    .add(super::_entities::email_queue::Column::NextRetryAt.lte(now_fixed)),
            )
            .order_by_asc(super::_entities::email_queue::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_by_recipient(
        db: &DatabaseConnection,
        recipient: &str,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::email_queue::Column::Recipient.eq(recipient))
            .order_by_desc(super::_entities::email_queue::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_by_type(
        db: &DatabaseConnection,
        email_type: &str,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::email_queue::Column::EmailType.eq(email_type))
            .order_by_desc(super::_entities::email_queue::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_failed(db: &DatabaseConnection) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::email_queue::Column::Status.eq("failed"))
            .order_by_desc(super::_entities::email_queue::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_by_status(
        db: &DatabaseConnection,
        status: &str,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::email_queue::Column::Status.eq(status))
            .order_by_desc(super::_entities::email_queue::Column::CreatedAt)
            .all(db)
            .await
    }

    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }

    pub fn is_sent(&self) -> bool {
        self.status == "sent"
    }

    pub fn is_failed(&self) -> bool {
        self.status == "failed"
    }

    pub fn can_retry(&self) -> bool {
        let retry_count = self.retry_count.unwrap_or(0);
        let max_retries = self.max_retries.unwrap_or(3);
        retry_count < max_retries && self.is_failed()
    }

    pub fn get_next_retry_delay(&self) -> chrono::Duration {
        let retry_count = self.retry_count.unwrap_or(0);
        match retry_count {
            0 => chrono::Duration::minutes(1),
            1 => chrono::Duration::minutes(5),
            2 => chrono::Duration::minutes(15),
            _ => chrono::Duration::minutes(30),
        }
    }
}

impl ActiveModel {
    pub async fn queue(
        db: &DatabaseConnection,
        params: &QueueEmailParams,
    ) -> Result<Model, DbErr> {
        let max_retries = params.max_retries.unwrap_or(3);

        let email = ActiveModel {
            recipient: sea_orm::ActiveValue::Set(params.recipient.clone()),
            recipient_name: sea_orm::ActiveValue::Set(params.recipient_name.clone()),
            email_type: sea_orm::ActiveValue::Set(params.email_type.clone()),
            subject: sea_orm::ActiveValue::Set(params.subject.clone()),
            html_content: sea_orm::ActiveValue::Set(params.html_content.clone()),
            text_content: sea_orm::ActiveValue::Set(params.text_content.clone()),
            metadata: sea_orm::ActiveValue::Set(params.metadata.clone()),
            status: sea_orm::ActiveValue::Set("pending".to_string()),
            retry_count: sea_orm::ActiveValue::Set(Some(0)),
            max_retries: sea_orm::ActiveValue::Set(Some(max_retries)),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(email)
    }

    pub async fn update_status(
        db: &DatabaseConnection,
        email_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        if !["pending", "sent", "failed"].contains(&status) {
            return Err(DbErr::RecordNotUpdated);
        }

        let email = Model::find_by_id(db, email_id).await?;
        let email = email.ok_or(DbErr::RecordNotFound("Email not found".to_string()))?;

        let mut active_model: ActiveModel = email.into();
        active_model.status = sea_orm::ActiveValue::Set(status.to_string());

        if status == "sent" {
            active_model.sent_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    pub async fn mark_sent(
        db: &DatabaseConnection,
        email_id: i32,
    ) -> Result<Model, DbErr> {
        Self::update_status(db, email_id, "sent").await
    }

    pub async fn mark_failed(
        db: &DatabaseConnection,
        email_id: i32,
        error: &str,
    ) -> Result<Model, DbErr> {
        let email = Model::find_by_id(db, email_id).await?;
        let email = email.ok_or(DbErr::RecordNotFound("Email not found".to_string()))?;

        let retry_count = email.retry_count.unwrap_or(0) + 1;
        let max_retries = email.max_retries.unwrap_or(3);

        let mut active_model: ActiveModel = email.into();
        active_model.status = sea_orm::ActiveValue::Set("failed".to_string());
        active_model.error_message = sea_orm::ActiveValue::Set(Some(error.to_string()));
        active_model.retry_count = sea_orm::ActiveValue::Set(Some(retry_count));

        let delay = match retry_count {
            1 => chrono::Duration::minutes(1),
            2 => chrono::Duration::minutes(5),
            3 => chrono::Duration::minutes(15),
            _ => chrono::Duration::minutes(30),
        };

        let next_retry = if retry_count < max_retries {
            Some((chrono::Utc::now() + delay).into())
        } else {
            None
        };

        active_model.next_retry_at = sea_orm::ActiveValue::Set(next_retry);

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    pub async fn reset_for_retry(
        db: &DatabaseConnection,
        email_id: i32,
    ) -> Result<Model, DbErr> {
        let email = Model::find_by_id(db, email_id).await?;
        let email = email.ok_or(DbErr::RecordNotFound("Email not found".to_string()))?;

        if !email.can_retry() {
            return Err(DbErr::RecordNotUpdated);
        }

        let mut active_model: ActiveModel = email.into();
        active_model.status = sea_orm::ActiveValue::Set("pending".to_string());
        active_model.error_message = sea_orm::ActiveValue::Set(None);
        active_model.next_retry_at = sea_orm::ActiveValue::Set(None);

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    pub async fn cleanup_sent(
        db: &DatabaseConnection,
        older_than_days: i64,
    ) -> Result<u64, DbErr> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days);
        let cutoff_fixed: chrono::DateTime<chrono::FixedOffset> = cutoff.into();

        let result = Entity::delete_many()
            .filter(super::_entities::email_queue::Column::Status.eq("sent"))
            .filter(super::_entities::email_queue::Column::CreatedAt.lt(cutoff_fixed))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn cleanup_failed(
        db: &DatabaseConnection,
        older_than_days: i64,
    ) -> Result<u64, DbErr> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days);
        let cutoff_fixed: chrono::DateTime<chrono::FixedOffset> = cutoff.into();

        let result = Entity::delete_many()
            .filter(super::_entities::email_queue::Column::Status.eq("failed"))
            .filter(super::_entities::email_queue::Column::CreatedAt.lt(cutoff_fixed))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn delete(db: &DatabaseConnection, email_id: i32) -> Result<(), DbErr> {
        let email = Model::find_by_id(db, email_id).await?;
        let email = email.ok_or(DbErr::RecordNotFound("Email not found".to_string()))?;

        let active_model: ActiveModel = email.into();
        active_model.delete(db).await?;
        Ok(())
    }
}

impl Entity {
    pub async fn get_stats(
        db: &DatabaseConnection,
    ) -> Result<(i64, i64, i64, i64), DbErr> {
        let total = Entity::find()
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let pending = Entity::find()
            .filter(super::_entities::email_queue::Column::Status.eq("pending"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let sent = Entity::find()
            .filter(super::_entities::email_queue::Column::Status.eq("sent"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let failed = Entity::find()
            .filter(super::_entities::email_queue::Column::Status.eq("failed"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, pending, sent, failed))
    }

    pub async fn get_stats_by_type(
        db: &DatabaseConnection,
        email_type: &str,
    ) -> Result<(i64, i64, i64), DbErr> {
        let total = Entity::find()
            .filter(super::_entities::email_queue::Column::EmailType.eq(email_type))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let sent = Entity::find()
            .filter(super::_entities::email_queue::Column::EmailType.eq(email_type))
            .filter(super::_entities::email_queue::Column::Status.eq("sent"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let failed = Entity::find()
            .filter(super::_entities::email_queue::Column::EmailType.eq(email_type))
            .filter(super::_entities::email_queue::Column::Status.eq("failed"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, sent, failed))
    }
}
