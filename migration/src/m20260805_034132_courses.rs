use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Courses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Courses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Courses::MentorId).integer().not_null())
                    .col(ColumnDef::new(Courses::Name).string().not_null())
                    .col(ColumnDef::new(Courses::Duration).string().not_null())
                    .col(ColumnDef::new(Courses::Description).string().not_null())
                    .col(
                        ColumnDef::new(Courses::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(Courses::EnrolledMenteesCount)
                            .integer()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Courses::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Courses::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_courses_mentor_id")
                            .from(Courses::Table, Courses::MentorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_courses_mentor_name_unique")
                    .table(Courses::Table)
                    .col(Courses::MentorId)
                    .col(Courses::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_courses_mentor_id")
                    .table(Courses::Table)
                    .col(Courses::MentorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_courses_status")
                    .table(Courses::Table)
                    .col(Courses::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Courses::Table).to_owned())
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
    MentorId,
    Name,
    Duration,
    Description,
    Status,
    EnrolledMenteesCount,
    CreatedAt,
    UpdatedAt,
}
