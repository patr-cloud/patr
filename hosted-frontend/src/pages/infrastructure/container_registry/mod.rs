mod container_registry_dashboard;
mod container_registry_item;
mod create_repository;
mod manage_repository;

pub use self::{
	container_registry_dashboard::*,
	container_registry_item::*,
	create_repository::*,
	manage_repository::*,
};
