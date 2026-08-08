use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LessonProgress::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LessonProgress::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LessonProgress::LessonId).integer().not_null())
                    .col(ColumnDef::new(LessonProgress::MenteeId).integer().not_null())
                    .col(
                        ColumnDef::new(LessonProgress::Completed)
                            .boolean()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(LessonProgress::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LessonProgress::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(LessonProgress::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_lesson_progress_lesson_id")
                            .from(LessonProgress::Table, LessonProgress::LessonId)
                            .to(Lessons::Table, Lessons::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_lesson_progress_mentee_id")
                            .from(LessonProgress::Table, LessonProgress::MenteeId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lesson_progress_unique")
                    .table(LessonProgress::Table)
                    .col(LessonProgress::LessonId)
                    .col(LessonProgress::MenteeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lesson_progress_lesson_id")
                    .table(LessonProgress::Table)
                    .col(LessonProgress::LessonId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lesson_progress_mentee_id")
                    .table(LessonProgress::Table)
                    .col(LessonProgress::MenteeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lesson_progress_completed")
                    .table(LessonProgress::Table)
                    .col(LessonProgress::Completed)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LessonProgress::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Lessons {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LessonProgress {
    Table,
    Id,
    LessonId,
    MenteeId,
    Completed,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}
