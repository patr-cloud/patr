//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

use super::sea_orm_active_enums::PermissionType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "role_resource_permissions_type")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub role_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub permission_id: Uuid,
	pub permission_type: PermissionType,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::permission::Entity",
		from = "Column::PermissionId",
		to = "super::permission::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Permission,
	#[sea_orm(
		belongs_to = "super::role::Entity",
		from = "Column::RoleId",
		to = "super::role::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Role,
	#[sea_orm(has_many = "super::role_resource_permissions_exclude::Entity")]
	RoleResourcePermissionsExclude,
	#[sea_orm(has_many = "super::role_resource_permissions_include::Entity")]
	RoleResourcePermissionsInclude,
}

impl Related<super::permission::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Permission.def()
	}
}

impl Related<super::role::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Role.def()
	}
}

impl Related<super::role_resource_permissions_exclude::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::RoleResourcePermissionsExclude.def()
	}
}

impl Related<super::role_resource_permissions_include::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::RoleResourcePermissionsInclude.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
