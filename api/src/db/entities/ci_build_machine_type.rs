//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ci_build_machine_type")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub cpu: i32,
	pub ram: i32,
	pub volume: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::ci_runner::Entity")]
	CiRunner,
}

impl Related<super::ci_runner::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiRunner.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
