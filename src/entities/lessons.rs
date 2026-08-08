use chrono::{DateTime, FixedOffset};
use sea_orm::QueryOrder;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "lessons")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub course_id: i32,
    pub mentor_id: i32,
    pub title: String,
    pub description: String,
    pub link: String,
    pub order_index: i32,
    pub status: String,
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

    pub fn find_by_course(course_id: i32) -> Select<Entity> {
        Self::find()
            .filter(Column::CourseId.eq(course_id))
            .order_by_asc(Column::OrderIndex)
    }
}
