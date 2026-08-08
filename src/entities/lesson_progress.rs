use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "lesson_progress")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub lesson_id: i32,
    pub mentee_id: i32,
    pub completed: Option<bool>,
    pub completed_at: Option<DateTime<FixedOffset>>,
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
    pub fn find_by_lesson_and_mentee(lesson_id: i32, mentee_id: i32) -> Select<Entity> {
        Self::find()
            .filter(Column::LessonId.eq(lesson_id))
            .filter(Column::MenteeId.eq(mentee_id))
    }
}
