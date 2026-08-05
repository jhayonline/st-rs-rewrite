use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Lessons::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Lessons::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Lessons::CourseId).integer().not_null())
                    .col(ColumnDef::new(Lessons::MentorId).integer().not_null())
                    .col(ColumnDef::new(Lessons::Title).string().not_null())
                    .col(ColumnDef::new(Lessons::Description).string().not_null())
                    .col(ColumnDef::new(Lessons::Link).string().not_null())
                    .col(
                        ColumnDef::new(Lessons::OrderIndex)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Lessons::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(Lessons::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Lessons::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_lessons_course_id")
                            .from(Lessons::Table, Lessons::CourseId)
                            .to(Courses::Table, Courses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_lessons_mentor_id")
                            .from(Lessons::Table, Lessons::MentorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lessons_course_id")
                    .table(Lessons::Table)
                    .col(Lessons::CourseId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lessons_mentor_id")
                    .table(Lessons::Table)
                    .col(Lessons::MentorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_lessons_order_index")
                    .table(Lessons::Table)
                    .col(Lessons::OrderIndex)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Lessons::Table).to_owned())
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
enum Lessons {
    Table,
    Id,
    CourseId,
    MentorId,
    Title,
    Description,
    Link,
    OrderIndex,
    Status,
    CreatedAt,
    UpdatedAt,
}
