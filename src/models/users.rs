use async_trait::async_trait;
use chrono::{offset::Local, Duration};
use loco_rs::{auth::jwt, hash, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};

pub const MAGIC_LINK_LENGTH: i8 = 32;
pub const MAGIC_LINK_EXPIRATION_MIN: i8 = 5;
pub const RESET_TOKEN_EXPIRATION_MIN: i64 = 30;

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    pub email: String,
    pub password: String,
    pub name: String,
    pub membership_category: String, // studet, professional, volunteer
    pub role: Option<String>, // mentee or mentor
    pub career_path: Option<String>,
    pub specialization: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProfileParams {
    pub name: Option<String>,
    pub career_path: Option<String>,
    pub specialization: Option<String>,
    pub community_link: Option<String>,
    pub contract_file_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateMembershipParams {
    pub membership_paid: bool,
    pub payment_reference: Option<String>,
    pub payment_date: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateStatusParams {
    pub status: String, // pending, approved, rejected, suspended
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateRoleParams {
    pub role: String, // admin, mentor, mentee
}

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    #[validate(email(message = "invalid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "Role is required"))]
    pub role: String,
    #[validate(length(min = 1, message = "Memebership category is required"))]
    pub membership_category: String,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
            role: self.role.as_ref().to_owned(),
            membership_category: self.membership_category.as_ref().to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;

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

#[async_trait]
impl Authenticable for Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        Self::find_by_api_key(db, api_key).await
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, claims_key).await
    }
}

impl Model {
    /// finds a user by the provided email
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, email)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided verification token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_verification_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::EmailVerificationToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                query::condition()
                    .eq(users::Column::MagicLinkToken, token)
                    .build(),
            )
            .one(db)
            .await?;

        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        if let Some(expired_at) = user.magic_link_expiration {
            if expired_at >= Local::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    token_expiration = expired_at.to_string(),
                    "magic token expired for the user."
                );
                Err(ModelError::msg("magic token expired"))
            }
        } else {
            tracing::error!(
                user_pid = user.pid.to_string(),
                "magic link expiration time not exists"
            );
            Err(ModelError::msg("expiration token not exists"))
        }
    }

    /// finds a user by the provided reset token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_reset_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ResetToken, token)
                    .build(),
            )
            .one(db)
            .await?;

        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        if let Some(sent_at) = user.reset_sent_at {
            if sent_at + Duration::minutes(RESET_TOKEN_EXPIRATION_MIN) >= Local::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    "reset token expired for the user."
                );
                Err(ModelError::msg("reset token expired"))
            }
        } else {
            tracing::error!(
                user_pid = user.pid.to_string(),
                "reset token sent time does not exist"
            );
            Err(ModelError::msg("reset token sent time not exists"))
        }
    }

    /// finds a user by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Pid, parse_uuid)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }


    /// finds a user by the provided api key
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    pub async fn find_by_role(db: &DatabaseConnection, role: &str) -> ModelResult<Vec<Self>> {
        let users = users::Entity::find()
            .filter(users::Column::Role.eq(role))
            .all(db)
            .await?;

        Ok(users)
    }

    pub async fn find_approved_mentors(db: &DatabaseConnection) -> ModelResult<Vec<Self>> {
        let users = users::Entity::find()
            .filter(users::Column::Role.eq("Mentor"))
            .filter(users::Column::Status.eq("approved"))
            .all(db)
            .await?;

        Ok(users)
    }

    pub async fn find_by_status(db: &DatabaseConnection, status: &str) -> ModelResult<Vec<Self>> {
        let users = users::Entity::find()
            .filter(users::Column::Status.eq(status))
            .all(db)
            .await?;
        Ok(users)
    }

    pub async fn find_pending_users(db: &DatabaseConnection) -> ModelResult<Vec<Self>> {
        Self::find_by_status(db, "pending").await
    }

    pub async fn find_membership_paid(db: &DatabaseConnection) -> ModelResult<Vec<Self>> {
        let users = users::Entity::find()
            .filter(users::Column::MembershipPaid.eq(true))
            .all(db)
            .await?;
        Ok(users)
    }

    /// Verifies whether the provided plain password matches the hashed password
    ///
    /// # Errors
    ///
    /// when could not verify password
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        hash::verify_password(password, &self.password)
    }

    /// Asynchronously creates a user with a password and saves it to the
    /// database.
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB
    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        if users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, &params.email)
                    .build(),
            )
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }

        let password_hash =
            hash::hash_password(&params.password).map_err(|e| ModelError::Any(e.into()))?;

        let role = params.role.clone().unwrap_or_else(|| "Mentee".to_string());

        let user = users::ActiveModel {
            email: ActiveValue::set(params.email.clone()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.clone()),
            role: ActiveValue::set(role.clone()),
            membership_category: ActiveValue::set(params.membership_category.clone()),
            career_path: ActiveValue::set(params.career_path.clone()),
            specialization: ActiveValue::set(params.specialization.clone()),
            status: ActiveValue::set("pending".to_string()),
            membership_enabled: ActiveValue::set(role == "Mentee" || role == "Mentor"),
            membership_paid: ActiveValue::set(false),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    pub async fn update_profile(
        db: &DatabaseConnection,
        user_id: i32,
        params: &UpdateProfileParams,
    ) -> ModelResult<Self> {
        let mut user = Entity::find_by_id(user_id).one(db).await?;
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;

        let mut active_model: ActiveModel = user.into();

        if let Some(name) = &params.name { active_model.name = ActiveValue::set(name.clone()); }
        if let Some(career_path) = &params.career_path { active_model.career_path = ActiveValue::set(Some(career_path.clone())); }
        if let Some(specialization) = &params.specialization { active_model.specialization = ActiveValue::set(Some(specialization.clone())); }
        if let Some(community_link) = &params.community_link { active_model.community_link = ActiveValue::set(Some(community_link.clone())); }
        if let Some(contract_file_url) = &params.contract_file_url { active_model.contract_file_url = ActiveValue::set(Some(contract_file_url.clone())); }

        active_model.update(db).await.map_err(ModelError::from)
    }

    pub async fn update_membership(
        db: &DatabaseConnection,
        user_id: i32,
        params: &UpdateMembershipParams,
    ) -> ModelResult<Self> {
        let mut user = Entity::find_by_id(user_id).one(db).await?;
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;

        let mut active_model: ActiveModel = user.into();
        active_model.membership_paid = ActiveValue::set(params.membership_paid);
        active_model.payment_reference = ActiveValue::set(params.payment_reference.clone());
        active_model.payment_date = ActiveValue::set(params.payment_date);

        active_model.update(db).await.map_err(ModelError::from)
    }

    pub async fn update_status(
        db: &DatabaseConnection,
        user_id: i32,
        status: &str,
    ) -> ModelResult<Self> {
        let mut user = Entity::find_by_id(user_id).one(db).await?;
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;

        let mut active_model: ActiveModel = user.into();
        active_model.status = ActiveValue::set(status.to_string());

        active_model.update(db).await.map_err(ModelError::from)
    }

    pub async fn update_role(
        db: &DatabaseConnection,
        user_id: i32,
        role: &str,
    ) -> ModelResult<Self> {
        let mut user = Entity::find_by_id(user_id).one(db).await?;
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;

        let mut active_model: ActiveModel = user.into();
        active_model.role = ActiveValue::set(role.to_string());

        active_model.update(db).await.map_err(ModelError::from)
    }

    pub async fn update_contract_file(
        db: &DatabaseConnection,
        user_id: i32,
        url: &str,
    ) -> ModelResult<Self> {
        let mut user = Entity::find_by_id(user_id).one(db).await?;
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;

        let mut active_model: ActiveModel = user.into();
        active_model.contract_file_url = ActiveValue::set(Some(url.to_string()));

        active_model.update(db).await.map_err(ModelError::from)
    }


    pub fn is_approved(&self) -> bool {
        self.status == "approved"
    }

    /// Check if user is pending
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }

    /// Check if user is rejected
    pub fn is_rejected(&self) -> bool {
        self.status == "rejected"
    }

    /// Check if user is suspended
    pub fn is_suspended(&self) -> bool {
        self.status == "suspended"
    }

    /// Check if user is a mentor
    pub fn is_mentor(&self) -> bool {
        self.role == "Mentor"
    }

    pub fn is_mentee(&self) -> bool {
        self.role == "Mentee"
    }

    /// Check if user is an admin
    pub fn is_admin(&self) -> bool {
        self.role == "Admin"
    }

    /// Check if user has active membership
    pub fn has_active_membership(&self) -> bool {
        self.membership_paid
    }

    /// Check if user needs to pay for membership
    pub fn needs_payment(&self) -> bool {
        self.membership_enabled && !self.membership_paid
    }

    pub fn get_membership_amount(&self) -> Option<f64> {
        self.membership_amount.map(|d| {
            d.to_string()
                .parse::<f64>()
                .unwrap_or(30.00)
        })
    }

    /// Creates a JWT
    ///
    /// # Errors
    ///
    /// when could not convert user claims to jwt token
    pub fn generate_jwt(&self, secret: &str, expiration: u64) -> ModelResult<String> {
        jwt::JWT::new(secret)
            .generate_token(expiration, self.pid.to_string(), Map::new())
            .map_err(ModelError::from)
    }
}

impl ActiveModel {
    /// Sets the email verification information for the user and
    /// updates it in the database.
    ///
    /// This method is used to record the timestamp when the email verification
    /// was sent and generate a unique verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        self.email_verification_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.email_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Sets the information for a reset password request,
    /// generates a unique reset password token, and updates it in the
    /// database.
    ///
    /// This method records the timestamp when the reset password token is sent
    /// and generates a unique token for the user.
    ///
    /// # Arguments
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.reset_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Records the verification time when a user verifies their
    /// email and updates it in the database.
    ///
    /// This method sets the timestamp when the user successfully verifies their
    /// email.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::set(Some(Local::now().into()));
        // Invalidate the verification token so it cannot be replayed after use.
        self.email_verification_token = ActiveValue::Set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Resets the current user password with a new password and
    /// updates it in the database.
    ///
    /// This method hashes the provided password and sets it as the new password
    /// for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error or could not hashed the given password
    pub async fn reset_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        self.password =
            ActiveValue::set(hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?);
        self.reset_token = ActiveValue::Set(None);
        self.reset_sent_at = ActiveValue::Set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Creates a magic link token for passwordless authentication.
    ///
    /// Generates a random token with a specified length and sets an expiration time
    /// for the magic link. This method is used to initiate the magic link authentication flow.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn create_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let random_str = hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        self.magic_link_token = ActiveValue::set(Some(random_str));
        self.magic_link_expiration = ActiveValue::set(Some(expired.into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Verifies and invalidates the magic link after successful authentication.
    ///
    /// Clears the magic link token and expiration time after the user has
    /// successfully authenticated using the magic link.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn clear_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.magic_link_token = ActiveValue::set(None);
        self.magic_link_expiration = ActiveValue::set(None);
        self.update(db).await.map_err(ModelError::from)
    }
}
