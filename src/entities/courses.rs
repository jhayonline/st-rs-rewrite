use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "courses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub mentor_id: i32,
    pub name: String,
    pub duration: String,
    pub description: String,
    pub status: String,
    pub enrolled_mentees_count: Option<i32>,
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

    pub fn find_active_by_mentor(mentor_id: i32) -> Select<Entity> {
        Self::find()
            .filter(Column::MentorId.eq(mentor_id))
            .filter(Column::Status.eq("active"))
    }
}
