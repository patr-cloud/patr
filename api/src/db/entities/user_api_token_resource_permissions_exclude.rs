//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

use super::sea_orm_active_enums::PermissionType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_api_token_resource_permissions_exclude")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub token_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub workspace_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub permission_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub resource_id: Uuid,
	pub permission_type: PermissionType,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::resource::Entity",
		from = "Column::ResourceId",
		to = "super::resource::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Resource,
	#[sea_orm(
		belongs_to = "super::user_api_token_resource_permissions_type::Entity",
		from = "Column::TokenId",
		to = "super::user_api_token_resource_permissions_type::Column::PermissionId",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	UserApiTokenResourcePermissionsType,
}

impl Related<super::resource::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Resource.def()
	}
}

impl Related<super::user_api_token_resource_permissions_type::Entity>
	for Entity
{
	fn to() -> RelationDef {
		Relation::UserApiTokenResourcePermissionsType.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}