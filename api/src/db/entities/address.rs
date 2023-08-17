//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "address")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub first_name: String,
	#[sea_orm(column_type = "Text")]
	pub last_name: String,
	#[sea_orm(column_type = "Text")]
	pub address_line_1: String,
	#[sea_orm(column_type = "Text", nullable)]
	pub address_line_2: Option<String>,
	#[sea_orm(column_type = "Text", nullable)]
	pub address_line_3: Option<String>,
	#[sea_orm(column_type = "Text")]
	pub city: String,
	#[sea_orm(column_type = "Text")]
	pub state: String,
	#[sea_orm(column_type = "Text")]
	pub zip: String,
	#[sea_orm(column_type = "Text")]
	pub country: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}