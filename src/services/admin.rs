use crate::entities::users;
use crate::models::admin::{
    CreateUserRequest, GetUsersRequest, MentorAssignment, MentorAssignmentDetail,
    MentorAssignmentsResponse, MentorResponse, StatsResponse, UpdateUserRequest, UserAdminResponse,
    UserListResponse,
};
use crate::utils::error::ApiError;
use crate::utils::validation;

use bcrypt::{DEFAULT_COST, hash};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive; // Add this import
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct AdminService;

impl AdminService {
    /// Get users with pagination and filtering
    pub async fn get_users(
        db: &Arc<DatabaseConnection>,
        req: &GetUsersRequest,
    ) -> Result<UserListResponse, ApiError> {
        let page = req.page.unwrap_or(1);
        let per_page = req.per_page.unwrap_or(10);
        let offset = (page - 1) * per_page;

        // Build the query
        let mut query = users::Entity::find();

        // Apply filters
        if let Some(search) = &req.search {
            if !search.is_empty() {
                let search_pattern = format!("%{}%", search);
                query = query.filter(
                    Condition::any()
                        .add(users::Column::Name.like(&search_pattern))
                        .add(users::Column::Email.like(&search_pattern)),
                );
            }
        }

        if let Some(status) = &req.status {
            if status != "all" {
                query = query.filter(users::Column::Status.eq(status));
            }
        }

        if let Some(role) = &req.role {
            if role != "all" {
                query = query.filter(users::Column::Role.eq(role));
            }
        }

        if let Some(category) = &req.membership_category {
            if category != "all" {
                query = query.filter(users::Column::MembershipCategory.eq(category));
            }
        }

        // Get total count
        let total_count = query.clone().count(db.as_ref()).await?;

        // Get paginated users
        let users = query
            .order_by_desc(users::Column::CreatedAt)
            .limit(per_page)
            .offset(offset)
            .all(db.as_ref())
            .await?;

        let user_responses: Vec<UserAdminResponse> = users
            .into_iter()
            .map(|user| UserAdminResponse {
                id: user.id,
                email: user.email,
                full_name: user.name,
                membership_category: user.membership_category,
                career_path: user.career_path,
                role: user.role,
                status: user.status,
                specialization: user.specialization,
                contract_file_url: user.contract_file_url,
                membership_enabled: user.membership_enabled,
                membership_amount: user.membership_amount.map(|d| d.to_string()),
                membership_paid: user.membership_paid,
                payment_reference: user.payment_reference,
                payment_date: user.payment_date,
                community_link: user.community_link,
                created_at: user.created_at,
                updated_at: user.updated_at,
            })
            .collect();

        let total_pages = (total_count as f64 / per_page as f64).ceil() as u64;

        Ok(UserListResponse {
            users: user_responses,
            total_count: total_count.try_into().unwrap_or(0),
            page,
            per_page,
            total_pages,
        })
    }

    /// Create a new user (admin only)
    pub async fn create_user(
        db: &Arc<DatabaseConnection>,
        req: &CreateUserRequest,
    ) -> Result<UserAdminResponse, ApiError> {
        // Validate email
        if !validation::validate_email(&req.email) {
            return Err(ApiError::bad_request("Invalid email format"));
        }

        // Validate password
        if let Err(e) = validation::validate_password(&req.password) {
            return Err(ApiError::bad_request(e));
        }

        // Validate role
        if !validation::validate_role(&req.role) {
            return Err(ApiError::bad_request("Invalid role"));
        }

        // Check if email exists
        let existing = users::Entity::find()
            .filter(users::Column::Email.eq(&req.email))
            .one(db.as_ref())
            .await?;

        if existing.is_some() {
            return Err(ApiError::conflict("Email already exists"));
        }

        let password_hash = hash(&req.password, DEFAULT_COST)?;

        let status = req.status.clone().unwrap_or_else(|| "pending".to_string());
        let membership_enabled = req
            .membership_enabled
            .unwrap_or_else(|| req.role == "Mentee" || req.role == "Mentor");
        let membership_amount = req.membership_amount.unwrap_or(30.0);

        let user = users::ActiveModel {
            email: ActiveValue::Set(req.email.clone()),
            password: ActiveValue::Set(password_hash),
            name: ActiveValue::Set(req.full_name.clone()),
            role: ActiveValue::Set(req.role.clone()),
            membership_category: ActiveValue::Set(req.membership_category.clone()),
            career_path: ActiveValue::Set(req.career_path.clone()),
            specialization: ActiveValue::Set(if req.role == "Mentor" {
                req.career_path.clone()
            } else {
                None
            }),
            status: ActiveValue::Set(status),
            membership_enabled: ActiveValue::Set(membership_enabled),
            membership_amount: ActiveValue::Set(
                Decimal::from_f64(membership_amount)
                    .map(Some)
                    .unwrap_or(None),
            ),
            membership_paid: ActiveValue::Set(false),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await?;

        Ok(UserAdminResponse {
            id: user.id,
            email: user.email,
            full_name: user.name,
            membership_category: user.membership_category,
            career_path: user.career_path,
            role: user.role,
            status: user.status,
            specialization: user.specialization,
            contract_file_url: user.contract_file_url,
            membership_enabled: user.membership_enabled,
            membership_amount: user.membership_amount.map(|d| d.to_string()),
            membership_paid: user.membership_paid,
            payment_reference: user.payment_reference,
            payment_date: user.payment_date,
            community_link: user.community_link,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// Update a user (admin only)
    pub async fn update_user(
        db: &Arc<DatabaseConnection>,
        req: &UpdateUserRequest,
    ) -> Result<UserAdminResponse, ApiError> {
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| ApiError::bad_request("Invalid user ID format"))?;

        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(user_id))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;

        let mut active_user: users::ActiveModel = user.into();

        if let Some(full_name) = &req.full_name {
            active_user.name = ActiveValue::Set(full_name.clone());
        }

        if let Some(email) = &req.email {
            if !validation::validate_email(email) {
                return Err(ApiError::bad_request("Invalid email format"));
            }
            // Check if email is taken by another user
            let existing = users::Entity::find()
                .filter(users::Column::Email.eq(email))
                .filter(users::Column::Pid.ne(user_id))
                .one(db.as_ref())
                .await?;

            if existing.is_some() {
                return Err(ApiError::conflict("Email already exists"));
            }
            active_user.email = ActiveValue::Set(email.clone());
        }

        if let Some(password) = &req.password {
            if let Err(e) = validation::validate_password(password) {
                return Err(ApiError::bad_request(e));
            }
            let password_hash = hash(password, DEFAULT_COST)?;
            active_user.password = ActiveValue::Set(password_hash);
        }

        if let Some(category) = &req.membership_category {
            if !validation::validate_membership_category(category) {
                return Err(ApiError::bad_request("Invalid membership category"));
            }
            active_user.membership_category = ActiveValue::Set(category.clone());
        }

        if let Some(career_path) = &req.career_path {
            active_user.career_path = ActiveValue::Set(Some(career_path.clone()));
        }

        if let Some(role) = &req.role {
            if !validation::validate_role(role) {
                return Err(ApiError::bad_request("Invalid role"));
            }
            active_user.role = ActiveValue::Set(role.clone());
            if role == "Mentor" {
                active_user.specialization = ActiveValue::Set(req.career_path.clone());
            }
        }

        if let Some(status) = &req.status {
            if !validation::validate_status(status) {
                return Err(ApiError::bad_request("Invalid status"));
            }
            active_user.status = ActiveValue::Set(status.clone());
        }

        if let Some(community_link) = &req.community_link {
            active_user.community_link = ActiveValue::Set(Some(community_link.clone()));
        }

        if let Some(membership_enabled) = &req.membership_enabled {
            active_user.membership_enabled = ActiveValue::Set(*membership_enabled);
        }

        if let Some(amount) = &req.membership_amount {
            active_user.membership_amount =
                ActiveValue::Set(Decimal::from_f64(*amount).map(Some).unwrap_or(None));
        }

        let updated_user = active_user.update(db.as_ref()).await?;

        Ok(UserAdminResponse {
            id: updated_user.id,
            email: updated_user.email,
            full_name: updated_user.name,
            membership_category: updated_user.membership_category,
            career_path: updated_user.career_path,
            role: updated_user.role,
            status: updated_user.status,
            specialization: updated_user.specialization,
            contract_file_url: updated_user.contract_file_url,
            membership_enabled: updated_user.membership_enabled,
            membership_amount: updated_user.membership_amount.map(|d| d.to_string()),
            membership_paid: updated_user.membership_paid,
            payment_reference: updated_user.payment_reference,
            payment_date: updated_user.payment_date,
            community_link: updated_user.community_link,
            created_at: updated_user.created_at,
            updated_at: updated_user.updated_at,
        })
    }

    /// Delete a user (admin only)
    pub async fn delete_user(db: &Arc<DatabaseConnection>, user_id: &str) -> Result<(), ApiError> {
        let user_pid = Uuid::parse_str(user_id)
            .map_err(|_| ApiError::bad_request("Invalid user ID format"))?;

        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(user_pid))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;

        let active_user: users::ActiveModel = user.into();
        active_user.delete(db.as_ref()).await?;

        Ok(())
    }

    /// Get statistics
    pub async fn get_stats(db: &Arc<DatabaseConnection>) -> Result<StatsResponse, ApiError> {
        let total = users::Entity::find()
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        let approved = users::Entity::find()
            .filter(users::Column::Status.eq("approved"))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        let pending = users::Entity::find()
            .filter(users::Column::Status.eq("pending"))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        let rejected = users::Entity::find()
            .filter(users::Column::Status.eq("rejected"))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        let suspended = users::Entity::find()
            .filter(users::Column::Status.eq("suspended"))
            .count(db.as_ref())
            .await?
            .try_into()
            .unwrap_or(0);

        Ok(StatsResponse {
            total,
            approved,
            pending,
            rejected,
            suspended,
        })
    }

    /// Get all approved mentors
    pub async fn get_mentors(
        db: &Arc<DatabaseConnection>,
    ) -> Result<Vec<MentorResponse>, ApiError> {
        let mentors = users::Entity::find()
            .filter(users::Column::Role.eq("Mentor"))
            .filter(users::Column::Status.eq("approved"))
            .all(db.as_ref())
            .await?;

        Ok(mentors
            .into_iter()
            .map(|user| MentorResponse {
                id: user.pid.to_string(),
                full_name: user.name,
                specialization: user.specialization.or(user.career_path),
            })
            .collect())
    }

    /// Get mentor assignments for a mentee
    pub async fn get_mentor_assignments(
        db: &Arc<DatabaseConnection>,
        mentee_id: &str,
    ) -> Result<MentorAssignmentsResponse, ApiError> {
        use crate::entities::mentor_mentee_relationships;

        let mentee_pid = Uuid::parse_str(mentee_id)
            .map_err(|_| ApiError::bad_request("Invalid mentee ID format"))?;

        let mentee = users::Entity::find()
            .filter(users::Column::Pid.eq(mentee_pid))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Mentee not found"))?;

        let relationships = mentor_mentee_relationships::Entity::find()
            .filter(mentor_mentee_relationships::Column::MenteeId.eq(mentee.id))
            .all(db.as_ref())
            .await?;

        let mut assignments = Vec::new();

        for rel in relationships {
            let mentor = users::Entity::find_by_id(rel.mentor_id)
                .one(db.as_ref())
                .await?;

            if let Some(mentor) = mentor {
                assignments.push(MentorAssignmentDetail {
                    id: rel.id.to_string(),
                    mentor: mentor.pid.to_string(),
                    mentor_name: mentor.name,
                });
            }
        }

        Ok(MentorAssignmentsResponse { assignments })
    }

    /// Update mentor assignments for a mentee
    pub async fn update_mentor_assignments(
        db: &Arc<DatabaseConnection>,
        mentee_id: &str,
        assignments: &[MentorAssignment],
    ) -> Result<(), ApiError> {
        use crate::entities::mentor_mentee_relationships;

        let mentee_pid = Uuid::parse_str(mentee_id)
            .map_err(|_| ApiError::bad_request("Invalid mentee ID format"))?;

        let mentee = users::Entity::find()
            .filter(users::Column::Pid.eq(mentee_pid))
            .one(db.as_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("Mentee not found"))?;

        // Get existing assignments
        let existing = mentor_mentee_relationships::Entity::find()
            .filter(mentor_mentee_relationships::Column::MenteeId.eq(mentee.id))
            .all(db.as_ref())
            .await?;

        // Build map of existing assignment IDs
        let existing_ids: Vec<i32> = existing.iter().map(|r| r.id).collect();
        let new_ids: Vec<String> = assignments
            .iter()
            .filter(|a| a.id.is_some())
            .map(|a| a.id.clone().unwrap())
            .collect();

        // Delete removed assignments
        for id in existing_ids {
            let id_str = id.to_string();
            if !new_ids.contains(&id_str) {
                let rel = mentor_mentee_relationships::Entity::find_by_id(id)
                    .one(db.as_ref())
                    .await?;
                if let Some(rel) = rel {
                    let active: mentor_mentee_relationships::ActiveModel = rel.into();
                    active.delete(db.as_ref()).await?;
                }
            }
        }

        // Add new assignments
        for assignment in assignments.iter().filter(|a| a.id.is_none()) {
            let mentor_pid = Uuid::parse_str(&assignment.mentor)
                .map_err(|_| ApiError::bad_request("Invalid mentor ID format"))?;

            let mentor = users::Entity::find()
                .filter(users::Column::Pid.eq(mentor_pid))
                .one(db.as_ref())
                .await?
                .ok_or_else(|| ApiError::not_found("Mentor not found"))?;

            mentor_mentee_relationships::ActiveModel {
                mentor_id: ActiveValue::Set(mentor.id),
                mentee_id: ActiveValue::Set(mentee.id),
                status: ActiveValue::Set("active".to_string()),
                progress_percentage: ActiveValue::Set(Some(0)),
                ..Default::default()
            }
            .insert(db.as_ref())
            .await?;
        }

        Ok(())
    }
}
