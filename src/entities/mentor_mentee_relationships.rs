use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mentor_mentee_relationships")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub mentor_id: i32,
    pub mentee_id: i32,
    pub status: String,
    pub assigned_date: DateTime<FixedOffset>,
    pub completion_date: Option<DateTime<FixedOffset>>,
    pub progress_percentage: Option<i32>,
    pub notes: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        use sea_orm::ActiveValue;
        let mut this = self;
        if insert {
            this.assigned_date = ActiveValue::Set(chrono::Utc::now().into());
            this.created_at = ActiveValue::Set(chrono::Utc::now().into());
        }
        this.updated_at = ActiveValue::Set(chrono::Utc::now().into());
        Ok(this)
    }
}

impl Entity {
    pub fn find_by_id(id: i32) -> Select<Entity> {
        Self::find().filter(Column::Id.eq(id))
    }

    pub fn find_by_mentor(mentor_id: i32) -> Select<Entity> {
        Self::find().filter(Column::MentorId.eq(mentor_id))
    }

    pub fn find_by_mentee(mentee_id: i32) -> Select<Entity> {
        Self::find().filter(Column::MenteeId.eq(mentee_id))
    }

    pub fn find_by_mentor_and_mentee(mentor_id: i32, mentee_id: i32) -> Select<Entity> {
        Self::find()
            .filter(Column::MentorId.eq(mentor_id))
            .filter(Column::MenteeId.eq(mentee_id))
    }

    pub fn find_active_by_mentor(mentor_id: i32) -> Select<Entity> {
        Self::find()
            .filter(Column::MentorId.eq(mentor_id))
            .filter(Column::Status.eq("active"))
    }
}
