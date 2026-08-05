use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Announcements::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Announcements::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Announcements::MentorId).integer().null())
                    .col(ColumnDef::new(Announcements::Title).string().not_null())
                    .col(ColumnDef::new(Announcements::Content).string().not_null())
                    .col(
                        ColumnDef::new(Announcements::TargetAudience)
                            .string()
                            .not_null()
                            .default("all"),
                    )
                    .col(ColumnDef::new(Announcements::TargetCourseId).integer().null())
                    .col(
                        ColumnDef::new(Announcements::Priority)
                            .string()
                            .not_null()
                            .default("normal"),
                    )
                    .col(
                        ColumnDef::new(Announcements::Published)
                            .boolean()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Announcements::PublishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Announcements::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Announcements::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Announcements::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_announcements_mentor_id")
                            .from(Announcements::Table, Announcements::MentorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_announcements_target_course_id")
                            .from(Announcements::Table, Announcements::TargetCourseId)
                            .to(Courses::Table, Courses::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_announcements_mentor_id")
                    .table(Announcements::Table)
                    .col(Announcements::MentorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_announcements_target_audience")
                    .table(Announcements::Table)
                    .col(Announcements::TargetAudience)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_announcements_published")
                    .table(Announcements::Table)
                    .col(Announcements::Published)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_announcements_target_course_id")
                    .table(Announcements::Table)
                    .col(Announcements::TargetCourseId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Announcements::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Courses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Announcements {
    Table,
    Id,
    MentorId,
    Title,
    Content,
    TargetAudience,
    TargetCourseId,
    Priority,
    Published,
    PublishedAt,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}
