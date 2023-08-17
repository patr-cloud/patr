//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

use super::sea_orm_active_enums::TokenPermissionType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_api_token_workspace_super_admin")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub token_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub user_id: Uuid,
	#[sea_orm(primary_key, auto_increment = false)]
	pub workspace_id: Uuid,
	pub token_permission_type: TokenPermissionType,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::user_api_token::Entity",
		from = "Column::TokenId",
		to = "super::user_api_token::Column::TokenId",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	UserApiToken,
	#[sea_orm(
		belongs_to = "super::user_api_token_workspace_permission_type::Entity",
		from = "Column::TokenId",
		to = "super::user_api_token_workspace_permission_type::Column::WorkspaceId",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	UserApiTokenWorkspacePermissionType,
	#[sea_orm(
		belongs_to = "super::workspace::Entity",
		from = "Column::WorkspaceId",
		to = "super::workspace::Column::SuperAdminId",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Workspace,
}

impl Related<super::user_api_token::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::UserApiToken.def()
	}
}

impl Related<super::user_api_token_workspace_permission_type::Entity>
	for Entity
{
	fn to() -> RelationDef {
		Relation::UserApiTokenWorkspacePermissionType.def()
	}
}

impl Related<super::workspace::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Workspace.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
