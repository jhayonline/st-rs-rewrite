use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};

pub use super::_entities::contract_files::{ActiveModel, Entity, Model};
pub type ContractFiles = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateContractFileParams {
    pub user_id: i32,
    pub file_name: String,
    pub file_size: i64,
    pub file_type: String,
    pub storage_provider: String, // s3, firebase, gcs, azure, other
    pub storage_key: String,
    pub storage_bucket: String,
    pub storage_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateContractFileParams {
    pub storage_url: Option<String>,
    pub upload_status: Option<String>, // pending, completed, failed
    pub uploaded_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractFileResponse {
    pub id: i32,
    pub user_id: i32,
    pub file_name: String,
    pub file_size: i64,
    pub file_type: String,
    pub storage_provider: String,
    pub storage_key: String,
    pub storage_bucket: String,
    pub storage_url: Option<String>,
    pub upload_status: String,
    pub uploaded_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for ContractFileResponse {
    fn from(file: Model) -> Self {
        Self {
            id: file.id,
            user_id: file.user_id,
            file_name: file.file_name,
            file_size: file.file_size,
            file_type: file.file_type,
            storage_provider: file.storage_provider,
            storage_key: file.storage_key,
            storage_bucket: file.storage_bucket,
            storage_url: file.storage_url,
            upload_status: file.upload_status,
            uploaded_at: file.uploaded_at,
            created_at: file.created_at,
            updated_at: file.updated_at,
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

    pub async fn find_by_user(db: &DatabaseConnection, user_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .order_by_desc(super::_entities::contract_files::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_by_user_and_status(
        db: &DatabaseConnection,
        user_id: i32,
        status: &str,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .filter(super::_entities::contract_files::Column::UploadStatus.eq(status))
            .order_by_desc(super::_entities::contract_files::Column::CreatedAt)
            .all(db)
            .await
    }

    pub async fn find_by_storage_key(
        db: &DatabaseConnection,
        provider: &str,
        key: &str,
    ) -> Result<Option<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::contract_files::Column::StorageProvider.eq(provider))
            .filter(super::_entities::contract_files::Column::StorageKey.eq(key))
            .one(db)
            .await
    }

    pub async fn find_completed_by_user(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Self::find_by_user_and_status(db, user_id, "completed").await
    }

    pub async fn find_pending_by_user(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Self::find_by_user_and_status(db, user_id, "pending").await
    }

    pub fn is_completed(&self) -> bool {
        self.upload_status == "completed"
    }

    pub fn is_pending(&self) -> bool {
        self.upload_status == "pending"
    }

    pub fn is_failed(&self) -> bool {
        self.upload_status == "failed"
    }
}

impl ActiveModel {
    pub async fn create(
        db: &DatabaseConnection,
        params: &CreateContractFileParams,
    ) -> Result<Model, DbErr> {
        use crate::models::_entities::users;
        
        let user = users::Entity::find_by_id(params.user_id)
            .one(db)
            .await?;
        
        let _user = user.ok_or(DbErr::RecordNotFound("User not found".to_string()))?;

        if !["s3", "firebase", "gcs", "azure", "other"].contains(&params.storage_provider.as_str()) {
            return Err(DbErr::RecordNotInserted);
        }

        if params.file_size <= 0 {
            return Err(DbErr::RecordNotInserted);
        }

        let file = ActiveModel {
            user_id: sea_orm::ActiveValue::Set(params.user_id),
            file_name: sea_orm::ActiveValue::Set(params.file_name.clone()),
            file_size: sea_orm::ActiveValue::Set(params.file_size),
            file_type: sea_orm::ActiveValue::Set(params.file_type.clone()),
            storage_provider: sea_orm::ActiveValue::Set(params.storage_provider.clone()),
            storage_key: sea_orm::ActiveValue::Set(params.storage_key.clone()),
            storage_bucket: sea_orm::ActiveValue::Set(params.storage_bucket.clone()),
            storage_url: sea_orm::ActiveValue::Set(params.storage_url.clone()),
            upload_status: sea_orm::ActiveValue::Set("pending".to_string()),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(file)
    }

    pub async fn update(
        db: &DatabaseConnection,
        file_id: i32,
        params: &UpdateContractFileParams,
    ) -> Result<Model, DbErr> {
        let file = Model::find_by_id(db, file_id).await?;
        let file = file.ok_or(DbErr::RecordNotFound("File not found".to_string()))?;

        let mut active_model: ActiveModel = file.into();

        if let Some(url) = &params.storage_url {
            active_model.storage_url = sea_orm::ActiveValue::Set(Some(url.clone()));
        }
        if let Some(status) = &params.upload_status {
            if !["pending", "completed", "failed"].contains(&status.as_str()) {
                return Err(DbErr::RecordNotUpdated);
            }
            active_model.upload_status = sea_orm::ActiveValue::Set(status.clone());
        }
        if let Some(uploaded_at) = &params.uploaded_at {
            active_model.uploaded_at = sea_orm::ActiveValue::Set(Some(*uploaded_at));
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    pub async fn mark_completed(
        db: &DatabaseConnection,
        file_id: i32,
        url: Option<String>,
    ) -> Result<Model, DbErr> {
        let params = UpdateContractFileParams {
            storage_url: url,
            upload_status: Some("completed".to_string()),
            uploaded_at: Some(chrono::Utc::now().into()),
        };
        Self::update(db, file_id, &params).await
    }

    pub async fn mark_failed(
        db: &DatabaseConnection,
        file_id: i32,
    ) -> Result<Model, DbErr> {
        let params = UpdateContractFileParams {
            storage_url: None,
            upload_status: Some("failed".to_string()),
            uploaded_at: None,
        };
        Self::update(db, file_id, &params).await
    }

    pub async fn mark_pending(
        db: &DatabaseConnection,
        file_id: i32,
    ) -> Result<Model, DbErr> {
        let params = UpdateContractFileParams {
            storage_url: None,
            upload_status: Some("pending".to_string()),
            uploaded_at: None,
        };
        Self::update(db, file_id, &params).await
    }

    pub async fn delete(db: &DatabaseConnection, file_id: i32) -> Result<(), DbErr> {
        let file = Model::find_by_id(db, file_id).await?;
        let file = file.ok_or(DbErr::RecordNotFound("File not found".to_string()))?;

        let active_model: ActiveModel = file.into();
        active_model.delete(db).await?;
        Ok(())
    }
}

impl Entity {
    pub async fn get_user_files_with_details(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        let result = Entity::find()
            .find_also_related(users::Entity)
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .order_by_desc(super::_entities::contract_files::Column::CreatedAt)
            .all(db)
            .await?;

        let mut files = Vec::new();
        for (file, user) in result {
            if let Some(user) = user {
                files.push((file, user));
            }
        }
        
        Ok(files)
    }

    pub async fn get_stats_by_user(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<(i64, i64, i64, i64), DbErr> {
        let total = Entity::find()
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let completed = Entity::find()
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .filter(super::_entities::contract_files::Column::UploadStatus.eq("completed"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let pending = Entity::find()
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .filter(super::_entities::contract_files::Column::UploadStatus.eq("pending"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let failed = Entity::find()
            .filter(super::_entities::contract_files::Column::UserId.eq(user_id))
            .filter(super::_entities::contract_files::Column::UploadStatus.eq("failed"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, completed, pending, failed))
    }
}
