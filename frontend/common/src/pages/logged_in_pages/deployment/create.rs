use crate::prelude::*;

/// The page to create a new deployment. This page allows users to set up
/// a new deployment by providing necessary configurations and options.
#[component]
pub fn CreateDeploymentPage() -> Element {
	rsx! {
		PageContainer { class: "bg-onboard" }
	}
}
