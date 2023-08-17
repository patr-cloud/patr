//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "coupon_code")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
	pub code: String,
	pub credits_in_cents: i64,
	pub expiry: Option<DateTimeWithTimeZone>,
	pub uses_remaining: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::user_to_sign_up::Entity")]
	UserToSignUp,
}

impl Related<super::user_to_sign_up::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::UserToSignUp.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}