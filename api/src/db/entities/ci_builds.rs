//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

use super::sea_orm_active_enums::CiBuildStatus;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ci_builds")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub repo_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub build_num: i64,
	#[sea_orm(column_type = "Text")]
	pub git_ref: String,
	#[sea_orm(column_type = "Text")]
	pub git_commit: String,
	pub status: CiBuildStatus,
	pub created: DateTimeWithTimeZone,
	pub finished: Option<DateTimeWithTimeZone>,
	#[sea_orm(column_type = "Text", nullable)]
	pub message: Option<String>,
	#[sea_orm(column_type = "Text", nullable)]
	pub author: Option<String>,
	#[sea_orm(column_type = "Text", nullable)]
	pub git_commit_message: Option<String>,
	#[sea_orm(column_type = "Text", nullable)]
	pub git_pr_title: Option<String>,
	pub started: Option<DateTimeWithTimeZone>,
	pub runner_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::ci_repos::Entity",
		from = "Column::RepoId",
		to = "super::ci_repos::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	CiRepos,
	#[sea_orm(
		belongs_to = "super::ci_runner::Entity",
		from = "Column::RunnerId",
		to = "super::ci_runner::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	CiRunner,
	#[sea_orm(has_many = "super::ci_steps::Entity")]
	CiSteps,
}

impl Related<super::ci_repos::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiRepos.def()
	}
}

impl Related<super::ci_runner::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiRunner.def()
	}
}

impl Related<super::ci_steps::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::CiSteps.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
