use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "course_enrollments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub course_id: i32,
    pub mentee_id: i32,
    pub enrolled_at: Option<DateTime<FixedOffset>>,
    pub completed_at: Option<DateTime<FixedOffset>>,
    pub progress_percentage: Option<i32>,
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
            this.enrolled_at = ActiveValue::Set(Some(chrono::Utc::now().into()));
            this.created_at = ActiveValue::Set(chrono::Utc::now().into());
        }
        this.updated_at = ActiveValue::Set(chrono::Utc::now().into());
        Ok(this)
    }
}

impl Entity {
    pub fn find_by_course_and_mentee(course_id: i32, mentee_id: i32) -> Select<Entity> {
        Self::find()
            .filter(Column::CourseId.eq(course_id))
            .filter(Column::MenteeId.eq(mentee_id))
    }
}
