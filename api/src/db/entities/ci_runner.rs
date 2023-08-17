//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ci_runner")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(column_type = "custom(\"citext\")")]
	pub name: String,
	pub workspace_id: Uuid,
	pub region_id: Uuid,
	pub build_machine_type_id: Uuid,
	pub deleted: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::ci_build_machine_type::Entity",
		from = "Column::BuildMachineTypeId",
		to = "super::ci_build_machine_type::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	CiBuildMachineType,
	#[sea_orm(has_many = "super::ci_builds::Entity")]
	CiBuilds,
	#[sea_orm(has_many = "super::ci_repos::Entity")]
	CiRepos,
	#[sea_orm(
		belongs_to = "super::region::Entity",
		from = "Column::RegionId",
		to = "super::region::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Region,
	#[sea_orm(
		belongs_to = "super::resource::Entity",
		from = "Column::Id",
		to = "super::resource::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Resource,
	#[sea_orm(
		belongs_to = "super::workspace::Entity",
		from = "Column::WorkspaceId",
		to = "super::workspace::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Workspace,
}

impl Related<super::ci_build_machine_type::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiBuildMachineType.def()
	}
}

impl Related<super::ci_builds::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiBuilds.def()
	}
}

impl Related<super::ci_repos::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiRepos.def()
	}
}

impl Related<super::region::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Region.def()
	}
}

impl Related<super::resource::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Resource.def()
	}
}

impl Related<super::workspace::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Workspace.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}