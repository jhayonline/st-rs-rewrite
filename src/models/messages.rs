use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use sea_orm::Condition;
use serde::{Deserialize, Serialize};

pub use super::_entities::messages::{ActiveModel, Entity, Model};
pub type Messages = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct SendMessageParams {
    pub sender_id: i32,
    pub recipient_id: i32,
    pub subject: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateMessageParams {
    pub subject: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageResponse {
    pub id: i32,
    pub sender_id: i32,
    pub recipient_id: i32,
    pub subject: Option<String>,
    pub content: String,
    pub read: bool,
    pub read_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for MessageResponse {
    fn from(msg: Model) -> Self {
        Self {
            id: msg.id,
            sender_id: msg.sender_id,
            recipient_id: msg.recipient_id,
            subject: msg.subject,
            content: msg.content,
            read: msg.read.unwrap_or(false),
            read_at: msg.read_at,
            created_at: msg.created_at,
            updated_at: msg.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageWithDetails {
    pub message: MessageResponse,
    pub sender_name: String,
    pub sender_email: String,
    pub recipient_name: String,
    pub recipient_email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConversationSummary {
    pub recipient_id: i32,
    pub recipient_name: String,
    pub last_message: MessageResponse,
    pub unread_count: i64,
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

    pub async fn find_sent_by(
        db: &DatabaseConnection,
        sender_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::messages::Column::SenderId.eq(sender_id))
            .order_by_desc(super::_entities::messages::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_received_by(
        db: &DatabaseConnection,
        recipient_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(recipient_id))
            .order_by_desc(super::_entities::messages::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_unread_by(
        db: &DatabaseConnection,
        recipient_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(recipient_id))
            .filter(super::_entities::messages::Column::Read.eq(false))
            .order_by_desc(super::_entities::messages::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn get_unread_count(
        db: &DatabaseConnection,
        recipient_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(recipient_id))
            .filter(super::_entities::messages::Column::Read.eq(false))
            .count(db)
            .await?;
        
        Ok(count.try_into().unwrap_or(0))
    }

    pub async fn find_conversation(
        db: &DatabaseConnection,
        user1_id: i32,
        user2_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(super::_entities::messages::Column::SenderId.eq(user1_id))
                            .add(super::_entities::messages::Column::RecipientId.eq(user2_id)),
                    )
                    .add(
                        Condition::all()
                            .add(super::_entities::messages::Column::SenderId.eq(user2_id))
                            .add(super::_entities::messages::Column::RecipientId.eq(user1_id)),
                    ),
            )
            .order_by_asc(super::_entities::messages::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn get_unread_from_sender(
        db: &DatabaseConnection,
        recipient_id: i32,
        sender_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(recipient_id))
            .filter(super::_entities::messages::Column::SenderId.eq(sender_id))
            .filter(super::_entities::messages::Column::Read.eq(false))
            .count(db)
            .await?;
        
        Ok(count.try_into().unwrap_or(0))
    }

    pub fn is_read(&self) -> bool {
        self.read.unwrap_or(false)
    }
}

impl ActiveModel {
    pub async fn send(
        db: &DatabaseConnection,
        params: &SendMessageParams,
    ) -> Result<Model, DbErr> {
        use crate::models::_entities::users;
        
        let sender = users::Entity::find_by_id(params.sender_id)
            .one(db)
            .await?;
        
        let _sender = sender.ok_or(DbErr::RecordNotFound("Sender not found".to_string()))?;

        let recipient = users::Entity::find_by_id(params.recipient_id)
            .one(db)
            .await?;
        
        let _recipient = recipient.ok_or(DbErr::RecordNotFound("Recipient not found".to_string()))?;

        if params.sender_id == params.recipient_id {
            return Err(DbErr::RecordNotInserted);
        }

        let message = ActiveModel {
            sender_id: sea_orm::ActiveValue::Set(params.sender_id),
            recipient_id: sea_orm::ActiveValue::Set(params.recipient_id),
            subject: sea_orm::ActiveValue::Set(params.subject.clone()),
            content: sea_orm::ActiveValue::Set(params.content.clone()),
            read: sea_orm::ActiveValue::Set(Some(false)),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(message)
    }

    pub async fn update(
        db: &DatabaseConnection,
        message_id: i32,
        params: &UpdateMessageParams,
    ) -> Result<Model, DbErr> {
        let message = Model::find_by_id(db, message_id).await?;
        let message = message.ok_or(DbErr::RecordNotFound("Message not found".to_string()))?;

        let mut active_model: ActiveModel = message.into();

        if let Some(subject) = &params.subject {
            active_model.subject = sea_orm::ActiveValue::Set(Some(subject.clone()));
        }
        if let Some(content) = &params.content {
            active_model.content = sea_orm::ActiveValue::Set(content.clone());
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    pub async fn mark_read(
        db: &DatabaseConnection,
        message_id: i32,
    ) -> Result<Model, DbErr> {
        let message = Model::find_by_id(db, message_id).await?;
        let message = message.ok_or(DbErr::RecordNotFound("Message not found".to_string()))?;

        let mut active_model: ActiveModel = message.into();
        active_model.read = sea_orm::ActiveValue::Set(Some(true));
        active_model.read_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    pub async fn mark_all_read_from_sender(
        db: &DatabaseConnection,
        recipient_id: i32,
        sender_id: i32,
    ) -> Result<Vec<Model>, DbErr> {
        let unread = Model::find_unread_by(db, recipient_id).await?;
        let from_sender = unread
            .into_iter()
            .filter(|msg| msg.sender_id == sender_id)
            .collect::<Vec<_>>();

        let mut updated = Vec::new();
        for msg in from_sender {
            let updated_msg = Self::mark_read(db, msg.id).await?;
            updated.push(updated_msg);
        }

        Ok(updated)
    }

    pub async fn mark_all_read(
        db: &DatabaseConnection,
        recipient_id: i32,
    ) -> Result<Vec<Model>, DbErr> {
        let unread = Model::find_unread_by(db, recipient_id).await?;

        let mut updated = Vec::new();
        for msg in unread {
            let updated_msg = Self::mark_read(db, msg.id).await?;
            updated.push(updated_msg);
        }

        Ok(updated)
    }

    pub async fn delete(db: &DatabaseConnection, message_id: i32) -> Result<(), DbErr> {
        let message = Model::find_by_id(db, message_id).await?;
        let message = message.ok_or(DbErr::RecordNotFound("Message not found".to_string()))?;

        let active_model: ActiveModel = message.into();
        active_model.delete(db).await?;
        Ok(())
    }

    pub async fn delete_conversation(
        db: &DatabaseConnection,
        user1_id: i32,
        user2_id: i32,
    ) -> Result<(), DbErr> {
        let messages = Model::find_conversation(db, user1_id, user2_id).await?;

        for msg in messages {
            let active_model: ActiveModel = msg.into();
            active_model.delete(db).await?;
        }

        Ok(())
    }
}

impl Entity {
    pub async fn find_with_details(
        db: &DatabaseConnection,
        message_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::users::Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        let message = Model::find_by_id(db, message_id).await?;
        let message = message.ok_or(DbErr::RecordNotFound("Message not found".to_string()))?;

        let sender = users::Entity::find_by_id(message.sender_id)
            .one(db)
            .await?;
        let recipient = users::Entity::find_by_id(message.recipient_id)
            .one(db)
            .await?;

        if let (Some(sender), Some(recipient)) = (sender, recipient) {
            Ok(Some((message, sender, recipient)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_conversations(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Vec<ConversationSummary>, DbErr> {
        use crate::models::_entities::users;
        
        let sent = Entity::find()
            .filter(super::_entities::messages::Column::SenderId.eq(user_id))
            .all(db)
            .await?;

        let received = Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(user_id))
            .all(db)
            .await?;

        let mut recipient_ids = std::collections::HashSet::new();
        
        for msg in sent {
            recipient_ids.insert(msg.recipient_id);
        }
        
        for msg in received {
            recipient_ids.insert(msg.sender_id);
        }

        let mut summaries = Vec::new();
        for recipient_id in recipient_ids {
            let conversation = Model::find_conversation(db, user_id, recipient_id).await?;
            let last_message = conversation
                .last()
                .ok_or_else(|| DbErr::RecordNotFound("No messages found".to_string()))?;

            let recipient = users::Entity::find_by_id(recipient_id)
                .one(db)
                .await?;
            
            if let Some(recipient) = recipient {
                let unread_count = Model::get_unread_from_sender(db, user_id, recipient_id).await?;

                summaries.push(ConversationSummary {
                    recipient_id,
                    recipient_name: recipient.name,
                    last_message: last_message.clone().into(),
                    unread_count,
                });
            }
        }

        summaries.sort_by(|a, b| {
            b.last_message.created_at.cmp(&a.last_message.created_at)
        });

        Ok(summaries)
    }

    pub async fn get_conversation_paginated(
        db: &DatabaseConnection,
        user1_id: i32,
        user2_id: i32,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<Model>, i64), DbErr> {
        let offset = (page - 1) * per_page;

        let total = Entity::find()
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(super::_entities::messages::Column::SenderId.eq(user1_id))
                            .add(super::_entities::messages::Column::RecipientId.eq(user2_id)),
                    )
                    .add(
                        Condition::all()
                            .add(super::_entities::messages::Column::SenderId.eq(user2_id))
                            .add(super::_entities::messages::Column::RecipientId.eq(user1_id)),
                    ),
            )
            .count(db)
            .await?;

        let messages = Entity::find()
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(super::_entities::messages::Column::SenderId.eq(user1_id))
                            .add(super::_entities::messages::Column::RecipientId.eq(user2_id)),
                    )
                    .add(
                        Condition::all()
                            .add(super::_entities::messages::Column::SenderId.eq(user2_id))
                            .add(super::_entities::messages::Column::RecipientId.eq(user1_id)),
                    ),
            )
            .order_by_desc(super::_entities::messages::Column::CreatedAt)
            .limit(per_page)
            .offset(offset)
            .all(db)
            .await?;

        Ok((messages, total.try_into().unwrap_or(0)))
    }

    pub async fn get_stats(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<(i64, i64, i64), DbErr> {
        let sent = Entity::find()
            .filter(super::_entities::messages::Column::SenderId.eq(user_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let received = Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(user_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let unread = Entity::find()
            .filter(super::_entities::messages::Column::RecipientId.eq(user_id))
            .filter(super::_entities::messages::Column::Read.eq(false))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((sent, received, unread))
    }
}
