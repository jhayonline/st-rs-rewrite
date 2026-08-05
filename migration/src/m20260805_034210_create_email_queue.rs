use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmailQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailQueue::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailQueue::Recipient).string().not_null())
                    .col(ColumnDef::new(EmailQueue::RecipientName).string().null())
                    .col(ColumnDef::new(EmailQueue::EmailType).string().not_null())
                    .col(ColumnDef::new(EmailQueue::Subject).string().not_null())
                    .col(ColumnDef::new(EmailQueue::HtmlContent).text().not_null())
                    .col(ColumnDef::new(EmailQueue::TextContent).text().null())
                    .col(ColumnDef::new(EmailQueue::Metadata).json().null())
                    .col(
                        ColumnDef::new(EmailQueue::Status)
                            .string()
                            .not_null()
                            .default("pending"), // pending, sent, failed
                    )
                    .col(ColumnDef::new(EmailQueue::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(EmailQueue::RetryCount)
                            .integer()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(EmailQueue::MaxRetries)
                            .integer()
                            .default(3),
                    )
                    .col(
                        ColumnDef::new(EmailQueue::NextRetryAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailQueue::SentAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(EmailQueue::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(EmailQueue::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_status")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_next_retry")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::NextRetryAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_email_type")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::EmailType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_created_at")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmailQueue::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EmailQueue {
    Table,
    Id,
    Recipient,
    RecipientName,
    EmailType,
    Subject,
    HtmlContent,
    TextContent,
    Metadata,
    Status,
    ErrorMessage,
    RetryCount,
    MaxRetries,
    NextRetryAt,
    SentAt,
    CreatedAt,
    UpdatedAt,
}
