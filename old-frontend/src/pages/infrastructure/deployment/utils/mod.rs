mod deployment_states;
mod error_states;
use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq)]
pub(super) struct DeploymentDashboardParams {
	pub page: Option<usize>,
}

pub use self::{deployment_states::*, error_states::*};
