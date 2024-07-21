use std::{collections::BTreeMap, rc::Rc};

use ev::MouseEvent;
use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::workspace::deployment::{
	DeploymentRegistry,
	EnvironmentVariableValue,
	ExposedPortType,
};

use crate::prelude::*;

mod details;
mod head;
mod running;
mod scale;

pub use self::{details::*, head::*, running::*, scale::*};

#[derive(Clone, Debug)]
pub struct DeploymentInfo {
	name: Option<String>,
	registry_name: Option<String>,
	image_tag: Option<String>,
	image_name: Option<String>,
	runner_id: Option<String>,
	machine_type: Option<String>,
	deploy_on_create: bool,
	deploy_on_push: bool,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
	ports: BTreeMap<StringifiedU16, ExposedPortType>,
	startup_probe: Option<(u16, String)>,
	liveness_probe: Option<(u16, String)>,
	environment_variables: BTreeMap<String, EnvironmentVariableValue>,
	volumes: BTreeMap<Uuid, String>,
}

#[derive(Clone)]
pub struct DetailsPageError {
	name: String,
	registry: String,
	image_name: String,
	image_tag: String,
	runner: String,
}

impl DetailsPageError {
	pub const fn new() -> Self {
		DetailsPageError {
			name: String::new(),
			runner: String::new(),
			image_tag: String::new(),
			image_name: String::new(),
			registry: String::new(),
		}
	}
}

#[derive(Clone)]
pub struct RunnerPageError {
	ports: String,
}

impl RunnerPageError {
	pub const fn new() -> Self {
		RunnerPageError {
			ports: String::new(),
		}
	}
}

#[component]
pub fn CreateDeployment() -> impl IntoView {
	let deployment_info = create_rw_signal(DeploymentInfo {
		name: None,
		registry_name: None,
		image_tag: None,
		image_name: None,
		runner_id: None,
		machine_type: None,
		deploy_on_create: false,
		deploy_on_push: false,
		min_horizontal_scale: None,
		max_horizontal_scale: None,
		startup_probe: None,
		liveness_probe: None,
		environment_variables: BTreeMap::new(),
		ports: BTreeMap::new(),
		volumes: BTreeMap::new(),
	});

	provide_context(deployment_info);

	let details_error = create_rw_signal(DetailsPageError::new());
	let runner_error = create_rw_signal(RunnerPageError::new());

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let create_deployment_action = create_server_action::<CreateDeploymentFn>();

	view! {
		<CreateDeploymentHead />
		<ContainerBody class="gap-md ofy-auto px-md">
			<ActionForm class="full-width" action={create_deployment_action}>
				<input type="hidden" name="workspace_id" value={current_workspace_id.get()} />
				<input type="hidden" name="access_token" value={access_token.get()} />
				<DeploymentDetails errors={details_error}  />
				<RunningDetails errors={runner_error} />
				<ScaleDeployment />

				<div class="fr-fe-ct gap-md full-width fit-wide-screen mx-auto mt-auto pt-md pb-xl px-md">
					<Link
						r#type={Variant::Link}
						style_variant={LinkStyleVariant::Plain}
						to="/deployment"
					>
						"BACK"
					</Link>

					<Link
						should_submit={true}
						style_variant={LinkStyleVariant::Contained}
						r#type={Variant::Button}
					>
						"CREATE"
					</Link>
				</div>
			</ActionForm>
		</ContainerBody>
	}
}
