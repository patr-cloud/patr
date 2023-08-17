//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

use super::sea_orm_active_enums::CiRepoStatus;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ci_repos")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub repo_owner: String,
	#[sea_orm(column_type = "Text")]
	pub repo_name: String,
	#[sea_orm(column_type = "Text")]
	pub clone_url: String,
	#[sea_orm(column_type = "Text", nullable, unique)]
	pub webhook_secret: Option<String>,
	pub status: CiRepoStatus,
	pub git_provider_id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub git_provider_repo_uid: String,
	pub runner_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::ci_builds::Entity")]
	CiBuilds,
	#[sea_orm(
		belongs_to = "super::ci_git_provider::Entity",
		from = "Column::GitProviderId",
		to = "super::ci_git_provider::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	CiGitProvider,
	#[sea_orm(
		belongs_to = "super::ci_runner::Entity",
		from = "Column::RunnerId",
		to = "super::ci_runner::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	CiRunner,
}

impl Related<super::ci_builds::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiBuilds.def()
	}
}

impl Related<super::ci_git_provider::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiGitProvider.def()
	}
}

impl Related<super::ci_runner::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiRunner.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}