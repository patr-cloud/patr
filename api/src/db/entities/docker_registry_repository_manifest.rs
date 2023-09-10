//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "docker_registry_repository_manifest")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub repository_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
	pub manifest_digest: String,
	pub size: i64,
	pub created: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::docker_registry_repository::Entity",
		from = "Column::RepositoryId",
		to = "super::docker_registry_repository::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	DockerRegistryRepository,
	#[sea_orm(has_many = "super::docker_registry_repository_tag::Entity")]
	DockerRegistryRepositoryTag,
}

impl Related<super::docker_registry_repository_tag::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::DockerRegistryRepositoryTag.def()
	}
}

impl Related<super::docker_registry_repository::Entity> for Entity {
	fn to() -> RelationDef {
		super::docker_registry_repository_tag::Relation::DockerRegistryRepository.def()
	}
	fn via() -> Option<RelationDef> {
		Some(
			super::docker_registry_repository_tag::Relation::DockerRegistryRepositoryManifest
				.def()
				.rev(),
		)
	}
}

impl ActiveModelBehavior for ActiveModel {}
