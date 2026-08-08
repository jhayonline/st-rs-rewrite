use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MentorMenteeRelationships::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MentorMenteeRelationships::MentorId).integer().not_null())
                    .col(ColumnDef::new(MentorMenteeRelationships::MenteeId).integer().not_null())
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::AssignedDate)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::CompletionDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::ProgressPercentage)
                            .integer()
                            .default(0),
                    )
                    .col(ColumnDef::new(MentorMenteeRelationships::Notes).string().null())
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(MentorMenteeRelationships::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mentor_mentee_relationships_mentor_id")
                            .from(MentorMenteeRelationships::Table, MentorMenteeRelationships::MentorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mentor_mentee_relationships_mentee_id")
                            .from(MentorMenteeRelationships::Table, MentorMenteeRelationships::MenteeId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mentor_mentee_unique")
                    .table(MentorMenteeRelationships::Table)
                    .col(MentorMenteeRelationships::MentorId)
                    .col(MentorMenteeRelationships::MenteeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mentor_mentee_relationships_mentor_id")
                    .table(MentorMenteeRelationships::Table)
                    .col(MentorMenteeRelationships::MentorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mentor_mentee_relationships_mentee_id")
                    .table(MentorMenteeRelationships::Table)
                    .col(MentorMenteeRelationships::MenteeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mentor_mentee_relationships_status")
                    .table(MentorMenteeRelationships::Table)
                    .col(MentorMenteeRelationships::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MentorMenteeRelationships::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum MentorMenteeRelationships {
    Table,
    Id,
    MentorId,
    MenteeId,
    Status,
    AssignedDate,
    CompletionDate,
    ProgressPercentage,
    Notes,
    CreatedAt,
    UpdatedAt,
}
