use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CourseEnrollments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CourseEnrollments::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CourseEnrollments::CourseId).integer().not_null())
                    .col(ColumnDef::new(CourseEnrollments::MenteeId).integer().not_null())
                    .col(
                        ColumnDef::new(CourseEnrollments::EnrolledAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CourseEnrollments::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CourseEnrollments::ProgressPercentage)
                            .integer()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CourseEnrollments::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(CourseEnrollments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CourseEnrollments::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_course_enrollments_course_id")
                            .from(CourseEnrollments::Table, CourseEnrollments::CourseId)
                            .to(Courses::Table, Courses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_course_enrollments_mentee_id")
                            .from(CourseEnrollments::Table, CourseEnrollments::MenteeId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_course_enrollments_unique")
                    .table(CourseEnrollments::Table)
                    .col(CourseEnrollments::CourseId)
                    .col(CourseEnrollments::MenteeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_course_enrollments_course_id")
                    .table(CourseEnrollments::Table)
                    .col(CourseEnrollments::CourseId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_course_enrollments_mentee_id")
                    .table(CourseEnrollments::Table)
                    .col(CourseEnrollments::MenteeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_course_enrollments_status")
                    .table(CourseEnrollments::Table)
                    .col(CourseEnrollments::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CourseEnrollments::Table).to_owned())
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
enum CourseEnrollments {
    Table,
    Id,
    CourseId,
    MenteeId,
    EnrolledAt,
    CompletedAt,
    ProgressPercentage,
    Status,
    CreatedAt,
    UpdatedAt,
}
