//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_to_sign_up")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub username: String,
	#[sea_orm(column_type = "Text")]
	pub password: String,
	pub first_name: String,
	pub last_name: String,
	pub recovery_email_local: Option<String>,
	pub recovery_email_domain_id: Option<Uuid>,
	pub recovery_phone_country_code: Option<String>,
	pub recovery_phone_number: Option<String>,
	#[sea_orm(column_type = "Text")]
	pub otp_hash: String,
	pub otp_expiry: DateTimeWithTimeZone,
	#[sea_orm(column_type = "Text", nullable)]
	pub coupon_code: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::coupon_code::Entity",
		from = "Column::CouponCode",
		to = "super::coupon_code::Column::Code",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	CouponCode,
	#[sea_orm(
		belongs_to = "super::personal_domain::Entity",
		from = "Column::RecoveryEmailDomainId",
		to = "super::personal_domain::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	PersonalDomain,
	#[sea_orm(
		belongs_to = "super::phone_number_country_code::Entity",
		from = "Column::RecoveryPhoneCountryCode",
		to = "super::phone_number_country_code::Column::CountryCode",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	PhoneNumberCountryCode,
}

impl Related<super::coupon_code::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CouponCode.def()
	}
}

impl Related<super::personal_domain::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::PersonalDomain.def()
	}
}

impl Related<super::phone_number_country_code::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::PhoneNumberCountryCode.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}