//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "docker_registry_repository")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub workspace_id: Uuid,
	#[sea_orm(column_type = "custom(\"citext\")")]
	pub name: String,
	pub deleted: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::deployment::Entity")]
	Deployment,
	#[sea_orm(has_many = "super::deployment_deploy_history::Entity")]
	DeploymentDeployHistory,
	#[sea_orm(has_many = "super::docker_registry_repository_manifest::Entity")]
	DockerRegistryRepositoryManifest,
	#[sea_orm(has_many = "super::docker_registry_repository_tag::Entity")]
	DockerRegistryRepositoryTag,
	#[sea_orm(
		belongs_to = "super::resource::Entity",
		from = "Column::Id",
		to = "super::resource::Column::OwnerId",
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

impl Related<super::deployment_deploy_history::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::DeploymentDeployHistory.def()
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

impl Related<super::deployment::Entity> for Entity {
	fn to() -> RelationDef {
		super::deployment_deploy_history::Relation::Deployment.def()
	}
	fn via() -> Option<RelationDef> {
		Some (super :: deployment_deploy_history :: Relation :: DockerRegistryRepository . def () . rev ())
	}
}

impl Related<super::docker_registry_repository_manifest::Entity> for Entity {
	fn to() -> RelationDef {
		super :: docker_registry_repository_tag :: Relation :: DockerRegistryRepositoryManifest . def ()
	}
	fn via() -> Option<RelationDef> {
		Some (super :: docker_registry_repository_tag :: Relation :: DockerRegistryRepository . def () . rev ())
	}
}

impl Related<super::docker_registry_repository_tag::Entity> for Entity {
	fn to() -> RelationDef {
		super :: docker_registry_repository_manifest :: Relation :: DockerRegistryRepositoryTag . def ()
	}
	fn via() -> Option<RelationDef> {
		Some (super :: docker_registry_repository_manifest :: Relation :: DockerRegistryRepository . def () . rev ())
	}
}

impl ActiveModelBehavior for ActiveModel {}