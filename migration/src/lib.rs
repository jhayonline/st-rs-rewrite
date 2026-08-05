#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20260805_034128_create_mentor_mentee_relationships;
mod m20260805_034132_courses;
mod m20260805_034137_create_course_enrollments;
mod m20260805_034141_lessons;
mod m20260805_034146_create_lesson_progress;
mod m20260805_034150_tasks;
mod m20260805_034154_create_task_submissions;
mod m20260805_034158_announcements;
mod m20260805_034202_messages;
mod m20260805_034206_create_contract_files;
mod m20260805_034210_create_email_queue;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20260805_034128_create_mentor_mentee_relationships::Migration),
            Box::new(m20260805_034132_courses::Migration),
            Box::new(m20260805_034137_create_course_enrollments::Migration),
            Box::new(m20260805_034141_lessons::Migration),
            Box::new(m20260805_034146_create_lesson_progress::Migration),
            Box::new(m20260805_034150_tasks::Migration),
            Box::new(m20260805_034154_create_task_submissions::Migration),
            Box::new(m20260805_034158_announcements::Migration),
            Box::new(m20260805_034202_messages::Migration),
            Box::new(m20260805_034206_create_contract_files::Migration),
            Box::new(m20260805_034210_create_email_queue::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}