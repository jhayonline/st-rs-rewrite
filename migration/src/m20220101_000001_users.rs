use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Users::Pid).uuid().not_null())
                    .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Password).string().not_null())
                    .col(ColumnDef::new(Users::ApiKey).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .col(ColumnDef::new(Users::ResetToken).string().null())
                    .col(ColumnDef::new(Users::ResetSentAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Users::EmailVerificationToken).string().null())
                    .col(ColumnDef::new(Users::EmailVerificationSentAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Users::EmailVerifiedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Users::MagicLinkToken).string().null())
                    .col(ColumnDef::new(Users::MagicLinkExpiration).timestamp_with_time_zone().null())
                    // SlintTech fields
                    .col(ColumnDef::new(Users::Role).string().not_null())
                    .col(ColumnDef::new(Users::MembershipCategory).string().not_null())
                    .col(ColumnDef::new(Users::CareerPath).string().null())
                    .col(ColumnDef::new(Users::Specialization).string().null())
                    .col(ColumnDef::new(Users::Status).string().not_null())
                    .col(ColumnDef::new(Users::MembershipEnabled).boolean().not_null().default(false))
                    .col(ColumnDef::new(Users::MembershipAmount).decimal().null().default(30.00))
                    .col(ColumnDef::new(Users::MembershipPaid).boolean().not_null().default(false))
                    .col(ColumnDef::new(Users::PaymentReference).string().null())
                    .col(ColumnDef::new(Users::PaymentDate).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Users::ContractFileUrl).string().null())
                    .col(ColumnDef::new(Users::CommunityLink).string().null())
                    .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_role")
                    .table(Users::Table)
                    .col(Users::Role)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_status")
                    .table(Users::Table)
                    .col(Users::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_membership_paid")
                    .table(Users::Table)
                    .col(Users::MembershipPaid)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Pid,
    Email,
    Password,
    ApiKey,
    Name,
    ResetToken,
    ResetSentAt,
    EmailVerificationToken,
    EmailVerificationSentAt,
    EmailVerifiedAt,
    MagicLinkToken,
    MagicLinkExpiration,
    // SlintTech fields
    Role,
    MembershipCategory,
    CareerPath,
    Specialization,
    Status,
    MembershipEnabled,
    MembershipAmount,
    MembershipPaid,
    PaymentReference,
    PaymentDate,
    ContractFileUrl,
    CommunityLink,
    CreatedAt,
    UpdatedAt,
}
