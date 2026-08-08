use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TaskSubmissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskSubmissions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskSubmissions::TaskId).integer().not_null())
                    .col(ColumnDef::new(TaskSubmissions::MenteeId).integer().not_null())
                    .col(ColumnDef::new(TaskSubmissions::SubmissionLink).string().null())
                    .col(ColumnDef::new(TaskSubmissions::SubmissionNotes).string().null())
                    .col(
                        ColumnDef::new(TaskSubmissions::SubmittedAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskSubmissions::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(TaskSubmissions::MentorFeedback).string().null())
                    .col(
                        ColumnDef::new(TaskSubmissions::ReviewedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(TaskSubmissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskSubmissions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_submissions_task_id")
                            .from(TaskSubmissions::Table, TaskSubmissions::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_submissions_mentee_id")
                            .from(TaskSubmissions::Table, TaskSubmissions::MenteeId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_submissions_unique")
                    .table(TaskSubmissions::Table)
                    .col(TaskSubmissions::TaskId)
                    .col(TaskSubmissions::MenteeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_submissions_task_id")
                    .table(TaskSubmissions::Table)
                    .col(TaskSubmissions::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_submissions_mentee_id")
                    .table(TaskSubmissions::Table)
                    .col(TaskSubmissions::MenteeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_submissions_status")
                    .table(TaskSubmissions::Table)
                    .col(TaskSubmissions::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskSubmissions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum TaskSubmissions {
    Table,
    Id,
    TaskId,
    MenteeId,
    SubmissionLink,
    SubmissionNotes,
    SubmittedAt,
    Status,
    MentorFeedback,
    ReviewedAt,
    CreatedAt,
    UpdatedAt,
}
