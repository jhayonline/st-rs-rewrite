use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ContractFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ContractFiles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ContractFiles::UserId).integer().not_null())
                    .col(ColumnDef::new(ContractFiles::FileName).string().not_null())
                    .col(ColumnDef::new(ContractFiles::FileSize).big_integer().not_null())
                    .col(ColumnDef::new(ContractFiles::FileType).string().not_null())
                    .col(ColumnDef::new(ContractFiles::StorageProvider).string().not_null())
                    .col(ColumnDef::new(ContractFiles::StorageKey).string().not_null())
                    .col(ColumnDef::new(ContractFiles::StorageBucket).string().not_null())
                    .col(ColumnDef::new(ContractFiles::StorageUrl).string().null())
                    .col(
                        ColumnDef::new(ContractFiles::UploadStatus)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(ContractFiles::UploadedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ContractFiles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ContractFiles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_contract_files_user_id")
                            .from(ContractFiles::Table, ContractFiles::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contract_files_storage_unique")
                    .table(ContractFiles::Table)
                    .col(ContractFiles::StorageProvider)
                    .col(ContractFiles::StorageKey)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contract_files_user_id")
                    .table(ContractFiles::Table)
                    .col(ContractFiles::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contract_files_storage_key")
                    .table(ContractFiles::Table)
                    .col(ContractFiles::StorageKey)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contract_files_storage_provider")
                    .table(ContractFiles::Table)
                    .col(ContractFiles::StorageProvider)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contract_files_upload_status")
                    .table(ContractFiles::Table)
                    .col(ContractFiles::UploadStatus)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ContractFiles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ContractFiles {
    Table,
    Id,
    UserId,
    FileName,
    FileSize,
    FileType,
    StorageProvider,
    StorageKey,
    StorageBucket,
    StorageUrl,
    UploadStatus,
    UploadedAt,
    CreatedAt,
    UpdatedAt,
}
