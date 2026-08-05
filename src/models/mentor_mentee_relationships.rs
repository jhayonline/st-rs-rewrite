use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};

pub use super::_entities::mentor_mentee_relationships::{ActiveModel, Entity, Model};
pub type MentorMenteeRelationships = Entity;

#[derive(Debug, Deserialize, Serialize)]
pub struct AssignMentorParams {
    pub mentor_id: i32,
    pub mentee_id: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkAssignParams {
    pub mentor_id: i32,
    pub mentee_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateAssignmentParams {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub progress_percentage: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RelationshipResponse {
    pub id: i32,
    pub mentor_id: i32,
    pub mentee_id: i32,
    pub status: String,
    pub assigned_date: chrono::DateTime<chrono::FixedOffset>,
    pub completion_date: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub progress_percentage: Option<i32>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<Model> for RelationshipResponse {
    fn from(rel: Model) -> Self {
        Self {
            id: rel.id,
            mentor_id: rel.mentor_id,
            mentee_id: rel.mentee_id,
            status: rel.status,
            assigned_date: rel.assigned_date,
            completion_date: rel.completion_date,
            progress_percentage: rel.progress_percentage,
            notes: rel.notes,
            created_at: rel.created_at,
            updated_at: rel.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RelationshipWithDetails {
    pub relationship: RelationshipResponse,
    pub mentor_name: String,
    pub mentor_email: String,
    pub mentee_name: String,
    pub mentee_email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MentorStats {
    pub total_mentees: i64,
    pub active_mentees: i64,
    pub completed_mentees: i64,
    pub average_progress: i32,
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
    /// Find relationship by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Self>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// Find relationship by mentor and mentee
    pub async fn find_by_mentor_and_mentee(
        db: &DatabaseConnection,
        mentor_id: i32,
        mentee_id: i32,
    ) -> Result<Option<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::MenteeId.eq(mentee_id))
            .one(db)
            .await
    }

    /// Find all relationships for a mentor
    pub async fn find_by_mentor(db: &DatabaseConnection, mentor_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::mentor_mentee_relationships::Column::AssignedDate)
            .all(db)
            .await
    }

    /// Find active relationships for a mentor
    pub async fn find_active_by_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .order_by_desc(super::_entities::mentor_mentee_relationships::Column::AssignedDate)
            .all(db)
            .await
    }

    /// Find all relationships for a mentee
    pub async fn find_by_mentee(db: &DatabaseConnection, mentee_id: i32) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MenteeId.eq(mentee_id))
            .order_by_desc(super::_entities::mentor_mentee_relationships::Column::AssignedDate)
            .all(db)
            .await
    }

    /// Find active relationships for a mentee
    pub async fn find_active_by_mentee(
        db: &DatabaseConnection,
        mentee_id: i32,
    ) -> Result<Vec<Self>, DbErr> {
        Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MenteeId.eq(mentee_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .order_by_desc(super::_entities::mentor_mentee_relationships::Column::AssignedDate)
            .all(db)
            .await
    }

    /// Check if a mentor is assigned to a mentee
    pub async fn is_mentor_assigned(
        db: &DatabaseConnection,
        mentor_id: i32,
        mentee_id: i32,
    ) -> Result<bool, DbErr> {
        let rel = Self::find_by_mentor_and_mentee(db, mentor_id, mentee_id).await?;
        Ok(rel.is_some())
    }

    /// Get mentee count for a mentor
    pub async fn get_mentee_count(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<i64, DbErr> {
        let count = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .count(db)
            .await?;
        
        Ok(count.try_into().unwrap_or(0))
    }

    /// Get statistics for a mentor
    pub async fn get_mentor_stats(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<MentorStats, DbErr> {
        let total = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let active = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        let completed = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("completed"))
            .count(db)
            .await?
            .try_into()
            .unwrap_or(0);

        // Calculate average progress
        let avg_progress = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .all(db)
            .await?;

        let avg = if !avg_progress.is_empty() {
            let sum: i32 = avg_progress
                .iter()
                .filter_map(|r| r.progress_percentage)
                .sum();
            sum / avg_progress.len() as i32
        } else {
            0
        };

        Ok(MentorStats {
            total_mentees: total,
            active_mentees: active,
            completed_mentees: completed,
            average_progress: avg,
        })
    }
}

impl ActiveModel {
    /// Assign a mentor to a mentee
    pub async fn assign_mentor(
        db: &DatabaseConnection,
        params: &AssignMentorParams,
    ) -> Result<Model, DbErr> {
        // Verify mentor exists
        use crate::models::_entities::users;
        
        let mentor = users::Entity::find_by_id(params.mentor_id)
            .one(db)
            .await?;
        
        let _mentor = mentor.ok_or(DbErr::RecordNotFound("Mentor not found".to_string()))?;

        // Verify mentee exists
        let mentee = users::Entity::find_by_id(params.mentee_id)
            .one(db)
            .await?;
        
        let _mentee = mentee.ok_or(DbErr::RecordNotFound("Mentee not found".to_string()))?;

        // Check if already assigned
        let existing = Model::find_by_mentor_and_mentee(
            db,
            params.mentor_id,
            params.mentee_id,
        )
        .await?;

        if existing.is_some() {
            return Err(DbErr::RecordNotInserted);
        }

        // Create relationship
        let relationship = ActiveModel {
            mentor_id: sea_orm::ActiveValue::Set(params.mentor_id),
            mentee_id: sea_orm::ActiveValue::Set(params.mentee_id),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            assigned_date: sea_orm::ActiveValue::Set(chrono::Utc::now().into()),
            notes: sea_orm::ActiveValue::Set(params.notes.clone()),
            progress_percentage: sea_orm::ActiveValue::Set(Some(0)),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(relationship)
    }

    /// Bulk assign a mentor to multiple mentees
    pub async fn bulk_assign(
        db: &DatabaseConnection,
        params: &BulkAssignParams,
    ) -> Result<Vec<Model>, DbErr> {
        let mut assigned = Vec::new();
        
        for mentee_id in &params.mentee_ids {
            let assign_params = AssignMentorParams {
                mentor_id: params.mentor_id,
                mentee_id: *mentee_id,
                notes: None,
            };
            
            match Self::assign_mentor(db, &assign_params).await {
                Ok(relationship) => assigned.push(relationship),
                Err(DbErr::RecordNotInserted) => {
                    // Already assigned, skip
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        Ok(assigned)
    }

    /// Update a relationship
    pub async fn update(
        db: &DatabaseConnection,
        relationship_id: i32,
        params: &UpdateAssignmentParams,
    ) -> Result<Model, DbErr> {
        let relationship = Model::find_by_id(db, relationship_id).await?;
        let relationship = relationship.ok_or(DbErr::RecordNotFound("Relationship not found".to_string()))?;

        let mut active_model: ActiveModel = relationship.into();

        if let Some(status) = &params.status {
            if !["active", "inactive", "completed", "paused"].contains(&status.as_str()) {
                return Err(DbErr::RecordNotUpdated);
            }
            active_model.status = sea_orm::ActiveValue::Set(status.clone());
            
            // If completing, set completion date
            if status == "completed" {
                active_model.completion_date = sea_orm::ActiveValue::Set(Some(chrono::Utc::now().into()));
            }
        }

        if let Some(notes) = &params.notes {
            active_model.notes = sea_orm::ActiveValue::Set(Some(notes.clone()));
        }

        if let Some(progress) = &params.progress_percentage {
            if !(0..=100).contains(progress) {
                return Err(DbErr::RecordNotUpdated);
            }
            active_model.progress_percentage = sea_orm::ActiveValue::Set(Some(*progress));
        }

        let updated = active_model.update(db).await?;
        Ok(updated)
    }

    /// Update relationship status
    pub async fn update_status(
        db: &DatabaseConnection,
        relationship_id: i32,
        status: &str,
    ) -> Result<Model, DbErr> {
        let params = UpdateAssignmentParams {
            status: Some(status.to_string()),
            notes: None,
            progress_percentage: None,
        };
        Self::update(db, relationship_id, &params).await
    }

    /// Update progress for a relationship
    pub async fn update_progress(
        db: &DatabaseConnection,
        relationship_id: i32,
        progress: i32,
    ) -> Result<Model, DbErr> {
        let params = UpdateAssignmentParams {
            status: None,
            notes: None,
            progress_percentage: Some(progress),
        };
        Self::update(db, relationship_id, &params).await
    }

    /// Complete a relationship
    pub async fn complete(
        db: &DatabaseConnection,
        relationship_id: i32,
    ) -> Result<Model, DbErr> {
        Self::update_status(db, relationship_id, "completed").await
    }

    /// Pause a relationship
    pub async fn pause(
        db: &DatabaseConnection,
        relationship_id: i32,
    ) -> Result<Model, DbErr> {
        Self::update_status(db, relationship_id, "paused").await
    }

    /// Activate a relationship
    pub async fn activate(
        db: &DatabaseConnection,
        relationship_id: i32,
    ) -> Result<Model, DbErr> {
        Self::update_status(db, relationship_id, "active").await
    }

    /// Remove a relationship (hard delete)
    pub async fn remove(db: &DatabaseConnection, relationship_id: i32) -> Result<(), DbErr> {
        let relationship = Model::find_by_id(db, relationship_id).await?;
        let relationship = relationship.ok_or(DbErr::RecordNotFound("Relationship not found".to_string()))?;

        let active_model: ActiveModel = relationship.into();
        active_model.delete(db).await?;
        Ok(())
    }

    /// Calculate progress for a relationship based on course completions
    pub async fn calculate_progress(
        db: &DatabaseConnection,
        relationship_id: i32,
    ) -> Result<i32, DbErr> {
        let relationship = Model::find_by_id(db, relationship_id).await?;
        let relationship = relationship.ok_or(DbErr::RecordNotFound("Relationship not found".to_string()))?;

        // Get all courses for this mentor
        use crate::models::_entities::courses;
        use crate::models::course_enrollments;
        
        let courses = courses::Entity::find()
            .filter(courses::Column::MentorId.eq(relationship.mentor_id))
            .filter(courses::Column::Status.eq("active"))
            .all(db)
            .await?;

        if courses.is_empty() {
            return Ok(0);
        }

        // Calculate average progress across all courses for this mentee
        let mut total_progress = 0;
        let mut count = 0;

        for course in courses {
            let enrollment = course_enrollments::Model::find_by_course_and_mentee(
                db,
                course.id,
                relationship.mentee_id,
            )
            .await?;

            if let Some(enrollment) = enrollment {
                if enrollment.status == "active" || enrollment.status == "completed" {
                    total_progress += enrollment.progress_percentage.unwrap_or(0);
                    count += 1;
                }
            }
        }

        let avg_progress = if count > 0 {
            total_progress / count
        } else {
            0
        };

        // Update the relationship progress
        Self::update_progress(db, relationship_id, avg_progress).await?;

        Ok(avg_progress)
    }
}

impl Entity {
    /// Get relationship with mentor and mentee details
    pub async fn find_with_details(
        db: &DatabaseConnection,
        relationship_id: i32,
    ) -> Result<Option<(Model, crate::models::_entities::users::Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        // First get the relationship
        let relationship = Model::find_by_id(db, relationship_id).await?;
        let relationship = relationship.ok_or(DbErr::RecordNotFound("Relationship not found".to_string()))?;

        // Then get both users
        let mentor = users::Entity::find_by_id(relationship.mentor_id)
            .one(db)
            .await?;
        let mentee = users::Entity::find_by_id(relationship.mentee_id)
            .one(db)
            .await?;
        
        if let (Some(mentor), Some(mentee)) = (mentor, mentee) {
            Ok(Some((relationship, mentor, mentee)))
        } else {
            Ok(None)
        }
    }

    /// Get all relationships for a mentor with mentee details
    pub async fn find_by_mentor_with_mentees(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        // Get all relationships for the mentor
        let relationships = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .order_by_desc(super::_entities::mentor_mentee_relationships::Column::AssignedDate)
            .all(db)
            .await?;

        let mut results = Vec::new();
        for rel in relationships {
            // Get the mentee user
            let mentee = users::Entity::find_by_id(rel.mentee_id)
                .one(db)
                .await?;
            
            if let Some(mentee) = mentee {
                results.push((rel, mentee));
            }
        }
        
        Ok(results)
    }

    /// Get all relationships for a mentee with mentor details
    pub async fn find_by_mentee_with_mentors(
        db: &DatabaseConnection,
        mentee_id: i32,
    ) -> Result<Vec<(Model, crate::models::_entities::users::Model)>, DbErr> {
        use crate::models::_entities::users;
        
        // Get all relationships for the mentee
        let relationships = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MenteeId.eq(mentee_id))
            .order_by_desc(super::_entities::mentor_mentee_relationships::Column::AssignedDate)
            .all(db)
            .await?;

        let mut results = Vec::new();
        for rel in relationships {
            // Get the mentor user
            let mentor = users::Entity::find_by_id(rel.mentor_id)
                .one(db)
                .await?;
            
            if let Some(mentor) = mentor {
                results.push((rel, mentor));
            }
        }
        
        Ok(results)
    }

    /// Get all mentors assigned to a mentee (with user details)
    pub async fn get_mentors_for_mentee(
        db: &DatabaseConnection,
        mentee_id: i32,
    ) -> Result<Vec<crate::models::_entities::users::Model>, DbErr> {
        use crate::models::_entities::users;
        
        // Get active relationships for the mentee
        let relationships = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MenteeId.eq(mentee_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .all(db)
            .await?;

        let mut mentors = Vec::new();
        for rel in relationships {
            // Get the mentor user
            let mentor = users::Entity::find_by_id(rel.mentor_id)
                .one(db)
                .await?;
            
            if let Some(mentor) = mentor {
                mentors.push(mentor);
            }
        }
        
        Ok(mentors)
    }

    /// Get all mentees assigned to a mentor (with user details)
    pub async fn get_mentees_for_mentor(
        db: &DatabaseConnection,
        mentor_id: i32,
    ) -> Result<Vec<crate::models::_entities::users::Model>, DbErr> {
        use crate::models::_entities::users;
        
        // Get active relationships for the mentor
        let relationships = Entity::find()
            .filter(super::_entities::mentor_mentee_relationships::Column::MentorId.eq(mentor_id))
            .filter(super::_entities::mentor_mentee_relationships::Column::Status.eq("active"))
            .all(db)
            .await?;

        let mut mentees = Vec::new();
        for rel in relationships {
            // Get the mentee user
            let mentee = users::Entity::find_by_id(rel.mentee_id)
                .one(db)
                .await?;
            
            if let Some(mentee) = mentee {
                mentees.push(mentee);
            }
        }
        
        Ok(mentees)
    }
}
