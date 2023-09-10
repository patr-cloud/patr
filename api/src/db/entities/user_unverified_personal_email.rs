//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_unverified_personal_email")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub local: String,
	#[sea_orm(primary_key, auto_increment = false)]
	pub domain_id: Uuid,
	pub user_id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub verification_token_hash: String,
	pub verification_token_expiry: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::personal_domain::Entity",
		from = "Column::DomainId",
		to = "super::personal_domain::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	PersonalDomain,
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::UserId",
		to = "super::user::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	User,
}

impl Related<super::personal_domain::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::PersonalDomain.def()
	}
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::User.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
