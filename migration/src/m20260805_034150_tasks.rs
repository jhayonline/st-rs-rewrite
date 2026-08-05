use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tasks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tasks::CourseId).integer().not_null())
                    .col(ColumnDef::new(Tasks::MentorId).integer().not_null())
                    .col(ColumnDef::new(Tasks::Title).string().not_null())
                    .col(ColumnDef::new(Tasks::Description).string().not_null())
                    .col(ColumnDef::new(Tasks::Requirements).json().null())
                    .col(ColumnDef::new(Tasks::Deadline).timestamp_with_time_zone().null())
                    .col(
                        ColumnDef::new(Tasks::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(Tasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Tasks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_course_id")
                            .from(Tasks::Table, Tasks::CourseId)
                            .to(Courses::Table, Courses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_mentor_id")
                            .from(Tasks::Table, Tasks::MentorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_course_id")
                    .table(Tasks::Table)
                    .col(Tasks::CourseId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_mentor_id")
                    .table(Tasks::Table)
                    .col(Tasks::MentorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_status")
                    .table(Tasks::Table)
                    .col(Tasks::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tasks::Table).to_owned())
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
enum Tasks {
    Table,
    Id,
    CourseId,
    MentorId,
    Title,
    Description,
    Requirements,
    Deadline,
    Status,
    CreatedAt,
    UpdatedAt,
}
