use chrono::{DateTime, FixedOffset};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pid: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub api_key: String,
    pub name: String,
    pub reset_token: Option<String>,
    pub reset_sent_at: Option<DateTime<FixedOffset>>,
    pub email_verification_token: Option<String>,
    pub email_verification_sent_at: Option<DateTime<FixedOffset>>,
    pub email_verified_at: Option<DateTime<FixedOffset>>,
    pub magic_link_token: Option<String>,
    pub magic_link_expiration: Option<DateTime<FixedOffset>>,
    pub role: String,
    pub membership_category: String,
    pub career_path: Option<String>,
    pub specialization: Option<String>,
    pub status: String,
    pub membership_enabled: bool,
    pub membership_amount: Option<Decimal>,
    pub membership_paid: bool,
    pub payment_reference: Option<String>,
    pub payment_date: Option<DateTime<FixedOffset>>,
    pub contract_file_url: Option<String>,
    pub community_link: Option<String>,
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
            this.pid = ActiveValue::Set(Uuid::new_v4());
            this.api_key = ActiveValue::Set(format!("lo-{}", Uuid::new_v4()));
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

    pub fn find_by_email(email: &str) -> Select<Entity> {
        Self::find().filter(Column::Email.eq(email))
    }

    pub fn find_by_role(role: &str) -> Select<Entity> {
        Self::find().filter(Column::Role.eq(role))
    }

    pub fn find_by_status(status: &str) -> Select<Entity> {
        Self::find().filter(Column::Status.eq(status))
    }

    pub fn find_approved_mentors() -> Select<Entity> {
        Self::find()
            .filter(Column::Role.eq("Mentor"))
            .filter(Column::Status.eq("approved"))
    }
}
