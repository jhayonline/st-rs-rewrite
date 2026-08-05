use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Messages::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Messages::SenderId).integer().not_null())
                    .col(ColumnDef::new(Messages::RecipientId).integer().not_null())
                    .col(ColumnDef::new(Messages::Subject).string().null())
                    .col(ColumnDef::new(Messages::Content).string().not_null())
                    .col(
                        ColumnDef::new(Messages::Read)
                            .boolean()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Messages::ReadAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Messages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Messages::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_sender_id")
                            .from(Messages::Table, Messages::SenderId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_recipient_id")
                            .from(Messages::Table, Messages::RecipientId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_sender_id")
                    .table(Messages::Table)
                    .col(Messages::SenderId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_recipient_id")
                    .table(Messages::Table)
                    .col(Messages::RecipientId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_read")
                    .table(Messages::Table)
                    .col(Messages::Read)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    SenderId,
    RecipientId,
    Subject,
    Content,
    Read,
    ReadAt,
    CreatedAt,
    UpdatedAt,
}
