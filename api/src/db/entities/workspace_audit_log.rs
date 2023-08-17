//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "workspace_audit_log")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub date: DateTimeWithTimeZone,
	#[sea_orm(column_type = "Text")]
	pub ip_address: String,
	pub workspace_id: Uuid,
	pub user_id: Option<Uuid>,
	pub login_id: Option<Uuid>,
	pub resource_id: Uuid,
	pub action: Uuid,
	pub request_id: Uuid,
	pub metadata: Json,
	pub patr_action: bool,
	pub success: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::permission::Entity",
		from = "Column::Action",
		to = "super::permission::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Permission,
	#[sea_orm(
		belongs_to = "super::resource::Entity",
		from = "Column::ResourceId",
		to = "super::resource::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Resource,
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::UserId",
		to = "super::user::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	User,
	#[sea_orm(
		belongs_to = "super::user_login::Entity",
		from = "Column::UserId",
		to = "super::user_login::Column::UserId",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	UserLogin,
	#[sea_orm(
		belongs_to = "super::workspace::Entity",
		from = "Column::WorkspaceId",
		to = "super::workspace::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	Workspace,
}

impl Related<super::permission::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Permission.def()
	}
}

impl Related<super::resource::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Resource.def()
	}
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::User.def()
	}
}

impl Related<super::user_login::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::UserLogin.def()
	}
}

impl Related<super::workspace::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Workspace.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
