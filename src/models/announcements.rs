use sea_orm::entity::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use sea_orm::Condition;
use serde::{Deserialize, Serialize};

pub use super::_entities::announcements::{ActiveModel, Entity, Model};
pub type Announcements = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAnnouncementParams {
    pub mentor_id: Option<i32>,
    pub title: String,
    pub content: String,
    pub target_audience: String, // all, mentees, mentors, specific
    pub target_course_id: Option<i32>,
    pub priority: Option<String>, // low, normal, high, urgent
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateAnnouncementParams {
    pub title: Option<String>,
    pub content: Option<String>,
    pub target_audience: Option<String>,
    pub target_course_id: Option<i32>,
    pub priority: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnnouncementResponse {
    pub id: i32,
    pub mentor_id: Option<i32>,
    pub title: String,
    pub content: String,
    pub target_audience: String,
    pub target_course_id: Option<i32>,
    pub priority: String,
    pub published: bool,
    pub published_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for AnnouncementResponse {
    fn from(announcement: Model) -> Self {
        Self {
            id: announcement.id,
            mentor_id: announcement.mentor_id,
            title: announcement.title,
            content: announcement.content,
            target_audience: announcement.target_audience,
            target_course_id: announcement.target_course_id,
            priority: announcement.priority,
            published: announcement.published.unwrap_or(false),
            published_at: announcement.published_at,
            expires_at: announcement.expires_at,
            created_at: announcement.created_at,
            updated_at: announcement.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnnouncementWithMentor {
    pub announcement: AnnouncementResponse,
    pub mentor_name: Option<String>,
    pub mentor_email: Option<String>,
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
    /// Find announcement by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find all published announcements
    pub async fn find_published(db: &DatabaseConnection) -> Result<Vec<Self>, DbErr> {
        let now = chrono::Utc::now();
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();

        Entity::find()
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::ExpiresAt.is_null())
                    .add(super::_entities::announcements::Column::ExpiresAt.gt(now_fixed)),
            )
            .order_by_desc(super::_entities::announcements::Column::Priority)
            .order_by_desc(super::_entities::announcements::Column::PublishedAt)
            .all(db)
            .await
    }

    /// Find published announcements for a specific audience
    pub async fn find_published_by_audience(
        db: &DatabaseConnection,
        audience: &str,
    ) -> Result<Vec<Self>, DbErr> {
        let now = chrono::Utc::now();
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();

        Entity::find()
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::TargetAudience.eq(audience))
                    .add(super::_entities::announcements::Column::TargetAudience.eq("all")),
            )
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::ExpiresAt.is_null())
                    .add(super::_entities::announcements::Column::ExpiresAt.gt(now_fixed)),
            )
            .order_by_desc(super::_entities::announcements::Column::Priority)
            .order_by_desc(super::_entities::announcements::Column::PublishedAt)
            .all(db)
            .await
    }

    /// Find announcements by mentor
    pub async fn find_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::announcements::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::announcements::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find announcements by course
    pub async fn find_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::announcements::Column::TargetCourseId.eq(course_id))
            .order_by_desc(super::_entities::announcements::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Find announcements by priority
    pub async fn find_by_priority(
        db: &DatabaseConnection,
        priority: &str,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::announcements::Column::Priority.eq(priority))
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .order_by_desc(super::_entities::announcements::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Get recent announcements (limit)
    pub async fn find_recent(
        db: &DatabaseConnection,
        limit: u64,
    ) -> Result<Vec<Self>, DbErr> {
        let now = chrono::Utc::now();
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();

        Entity::find()
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::ExpiresAt.is_null())
                    .add(super::_entities::announcements::Column::ExpiresAt.gt(now_fixed)),
            )
            .order_by_desc(super::_entities::announcements::Column::Priority)
            .order_by_desc(super::_entities::announcements::Column::PublishedAt)
            .limit(limit)
            .all(db)
            .await
    }

    /// Check if announcement is published
    pub fn is_published(&self) -> bool {
        self.published.unwrap_or(false)
    }

    /// Check if announcement is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
            expires_at < now
        } else {
            false
        }
    }

    /// Get priority level as integer for sorting
    pub fn priority_level(&self) -> i32 {
        match self.priority.as_str() {
            "urgent" => 4,
            "high" => 3,
            "normal" => 2,
            "low" => 1,
            _ => 0,
        }
    }
}

impl ActiveModel {
    /// Create a new announcement
    pub async fn create(
        db: &DatabaseConnection,
        params: &CreateAnnouncementParams,
    ) -> Result<Model, DbErr> {
        // Validate audience
        if !["all", "mentees", "mentors", "specific"].contains(&params.target_audience.as_str()) {
            return Err(DbErr::RecordNotInserted);
        }

        // Validate priority
        let priority = params.priority.clone().unwrap_or_else(|| "normal".to_string());
        if !["low", "normal", "high", "urgent"].contains(&priority.as_str()) {
            return Err(DbErr::RecordNotInserted);
        }

        // If mentor_id is provided, verify mentor exists
        if let Some(mentor_id) = params.mentor_id {
            use crate::models::_entities::users;
            
            let mentor = users::Entity::find_by_id(mentor_id)
                .one(db)
                .await?;
            
            let _mentor = mentor.ok_or(DbErr::RecordNotFound("Mentor not found".to_string()))?;
        }

        // If target_course_id is provided, verify course exists
        if let Some(course_id) = params.target_course_id {
            use crate::models::_entities::courses;
            
            let course = courses::Entity::find_by_id(course_id)
                .one(db)
                .await?;
            
            let _course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;
        }

        let announcement = ActiveModel {
            mentor_id: sea_orm::ActiveValue::Set(params.mentor_id),
            title: sea_orm::ActiveValue::Set(params.title.clone()),
            content: sea_orm::ActiveValue::Set(params.content.clone()),
            target_audience: sea_orm::ActiveValue::Set(params.target_audience.clone()),
            target_course_id: sea_orm::ActiveValue::Set(params.target_course_id),
            priority: sea_orm::ActiveValue::Set(priority),
            published: sea_orm::ActiveValue::Set(Some(false)),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(announcement)
    }

    /// Update an announcement
    pub async fn update(
        db: &DatabaseConnection,
        announcement_id: i32,
        params: &UpdateAnnouncementParams,
    ) -> Result<Model, DbErr> {
        let announcement = Model::find_by_id(db, announcement_id).await?;
        let announcement = announcement.ok_or(DbErr::RecordNotFound("Announcement not found".to_string()))?;

        let mut active_model: ActiveModel = announcement.into();

        if let Some(title) = &params.title {
            active_model.title = sea_orm::ActiveValue::Set(title.clone());
        }
        if let Some(content) = &params.content {
            active_model.content = sea_orm::ActiveValue::Set(content.clone());
        }
        if let Some(audience) = &params.target_audience {
            if !["all", "mentees", "mentors", "specific"].contains(&audience.as_str()) {
                return Err(DbErr::RecordNotUpdated);
            }
            active_model.target_audience = sea_orm::ActiveValue::Set(audience.clone());
        }
        if let Some(course_id) = &params.target_course_id {
            // Verify course exists
            use crate::models::_entities::courses;
            
            let course = courses::Entity::find_by_id(*course_id)
                .one(db)
                .await?;
            
            let _course = course.ok_or(DbErr::RecordNotFound("Course not found".to_string()))?;
            active_model.target_course_id = sea_orm::ActiveValue::Set(Some(*course_id));
        }
        if let Some(priority) = &params.priority {
            if !["low", "normal", "high", "urgent"].contains(&priority.as_str()) {
                return Err(DbErr::RecordNotUpdated);
            }
            active_model.priority = sea_orm::ActiveValue::Set(priority.clone());
        }
        if let Some(published) = &params.published {
            active_model.published = sea_orm::ActiveValue::Set(Some(*published));
            
            // If publishing, set published_at (check if it's already set)
            if *published && active_model.published_at.as_ref().is_none() {
                active_model.published_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
            }
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Publish an announcement
    pub async fn publish(
        db: &DatabaseConnection,
        announcement_id: i32,
    ) -> Result<Model, DbErr> {
        let params = UpdateAnnouncementParams {
            published: Some(true),
            title: None,
            content: None,
            target_audience: None,
            target_course_id: None,
            priority: None,
        };
        Self::update(db, announcement_id, &params).await
    }

    /// Unpublish an announcement
    pub async fn unpublish(
        db: &DatabaseConnection,
        announcement_id: i32,
    ) -> Result<Model, DbErr> {
        let announcement = Model::find_by_id(db, announcement_id).await?;
        let announcement = announcement.ok_or(DbErr::RecordNotFound("Announcement not found".to_string()))?;

        let mut active_model: ActiveModel = announcement.into();
        active_model.published = sea_orm::ActiveValue::Set(Some(false));

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Set expiration date
    pub async fn set_expiration(
        db: &DatabaseConnection,
        announcement_id: i32,
        expires_at: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<Model, DbErr> {
        let announcement = Model::find_by_id(db, announcement_id).await?;
        let announcement = announcement.ok_or(DbErr::RecordNotFound("Announcement not found".to_string()))?;

        let mut active_model: ActiveModel = announcement.into();
        active_model.expires_at = sea_orm::ActiveValue::Set(Some(expires_at));

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Delete an announcement (hard delete)
    pub async fn delete(db: &DatabaseConnection, announcement_id: i32) -> Result<(), DbErr> {
        let announcement = Model::find_by_id(db, announcement_id).await?;
        let announcement = announcement.ok_or(DbErr::RecordNotFound("Announcement not found".to_string()))?;

        let active_model: ActiveModel = announcement.into();
        active_model.delete(db).await?;
        Ok(())
    }
}

impl Entity {
    /// Get announcement with mentor details
    pub async fn find_with_mentor(
        db: &DatabaseConnection,
        announcement_id: i32,
    ) -> Result<Option<(Model, Option<crate::models::_entities::users::Model>)>, DbErr> {
        use crate::models::_entities::users;
        
        let announcement = Model::find_by_id(db, announcement_id).await?;
        let announcement = announcement.ok_or(DbErr::RecordNotFound("Announcement not found".to_string()))?;

        let mentor = if let Some(mentor_id) = announcement.mentor_id {
            users::Entity::find_by_id(mentor_id)
                .one(db)
                .await?
        } else {
            None
        };

        Ok(Some((announcement, mentor)))
    }

    /// Get published announcements for a specific course
    pub async fn find_published_by_course(
        db: &DatabaseConnection,
        course_id: i32,
    ) -> Result<Vec<Model>, DbErr> {
        let now = chrono::Utc::now();
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();

        Entity::find()
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::TargetAudience.eq("all"))
                    .add(super::_entities::announcements::Column::TargetAudience.eq("mentees"))
                    .add(super::_entities::announcements::Column::TargetCourseId.eq(course_id)),
            )
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::ExpiresAt.is_null())
                    .add(super::_entities::announcements::Column::ExpiresAt.gt(now_fixed)),
            )
            .order_by_desc(super::_entities::announcements::Column::Priority)
            .order_by_desc(super::_entities::announcements::Column::PublishedAt)
            .all(db)
            .await
    }

    /// Get announcement statistics for a mentor
    pub async fn get_stats_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<(i64, i64, i64), DbErr> {
        let total = Entity::find()
            .filter(super::_entities::announcements::Column::MentorId.eq(mentor_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let published = Entity::find()
            .filter(super::_entities::announcements::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let draft = Entity::find()
            .filter(super::_entities::announcements::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::announcements::Column::Published.eq(false))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        Ok((total, published, draft))
    }

    /// Get high priority announcements for a user
    pub async fn get_high_priority(
        db: &DatabaseConnection,
        audience: &str,
    ) -> Result<Vec<Model>, DbErr> {
        let now = chrono::Utc::now();
        let now_fixed: chrono::DateTime<chrono::FixedOffset> = now.into();

        Entity::find()
            .filter(super::_entities::announcements::Column::Published.eq(true))
            .filter(super::_entities::announcements::Column::Priority.is_in(vec!["high", "urgent"]))
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::TargetAudience.eq(audience))
                    .add(super::_entities::announcements::Column::TargetAudience.eq("all")),
            )
            .filter(
                Condition::any()
                    .add(super::_entities::announcements::Column::ExpiresAt.is_null())
                    .add(super::_entities::announcements::Column::ExpiresAt.gt(now_fixed)),
            )
            .order_by_desc(super::_entities::announcements::Column::Priority)
            .order_by_desc(super::_entities::announcements::Column::PublishedAt)
            .all(db)
            .await
    }
}
