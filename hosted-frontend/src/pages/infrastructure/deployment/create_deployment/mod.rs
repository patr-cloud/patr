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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
	#[default]
	Details,
	Running,
	Scaling,
}

impl Page {
	pub fn next(&self) -> Self {
		match self {
			Self::Details => Self::Running,
			Self::Running => Self::Scaling,
			Self::Scaling => Self::Scaling,
		}
	}

	pub fn back(&self) -> Self {
		match self {
			Self::Scaling => Self::Running,
			Self::Running => Self::Details,
			Self::Details => Self::Details,
		}
	}
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

	let page = create_rw_signal(Page::Details);

	let details_error = create_rw_signal(DetailsPageError::new());
	let runner_error = create_rw_signal(RunnerPageError::new());

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let on_submit = move |_: MouseEvent| {
		// let navigate =
		logging::log!("{:?}", deployment_info.get());
		spawn_local(async move {
			let resp = create_deployment(
				current_workspace_id.get(),
				access_token.get(),
				deployment_info.get().name.unwrap_or_default(),
				deployment_info.get().registry_name.unwrap_or_default(),
				deployment_info.get().image_name.unwrap_or_default(),
				deployment_info.get().image_tag.unwrap_or_default(),
				deployment_info.get().deploy_on_create,
				deployment_info.get().deploy_on_push,
				deployment_info
					.get()
					.min_horizontal_scale
					.unwrap_or_default(),
				deployment_info
					.get()
					.max_horizontal_scale
					.unwrap_or_default(),
				deployment_info.get().runner_id.unwrap_or_default(),
				deployment_info.get().startup_probe,
				deployment_info.get().liveness_probe,
				deployment_info.get().machine_type.unwrap_or_default(),
				deployment_info
					.get()
					.environment_variables
					.iter()
					.map(|x| (x.0.to_owned(), x.1.to_owned()))
					.collect::<Vec<_>>(),
				deployment_info
					.get()
					.volumes
					.iter()
					.map(|(id, dv)| (id.to_owned(), dv.to_owned()))
					.collect::<Vec<_>>(),
				deployment_info
					.get()
					.ports
					.iter()
					.map(|(port, port_type)| (port.to_owned(), port_type.to_owned()))
					.collect::<Vec<_>>(),
			)
			.await;
		})
	};

	view! {
		<CreateDeploymentHead />
		<ContainerBody class="gap-md ofy-auto px-md">
			{
				move || match page.get() {
					Page::Details => view! {
						<DeploymentDetails errors={details_error}  />
					}.into_view(),
					Page::Running => view! {
						<RunningDetails errors={runner_error} />
					}.into_view(),
					Page::Scaling => view! {
						<ScaleDeployment />
					}.into_view(),
				}
			}
			<div class="fr-fe-ct gap-md full-width fit-wide-screen mx-auto mt-auto pt-md pb-xl px-md">
				 <Show when={move || page.get() != Page::Details}>
					<Link
						on_click={Rc::new(move |_| {
							page.update(|x| *x = x.back());
						})}
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Plain}
					>
						"BACK"
					</Link>
				</Show>

				<Show
					when={move || page.get() == Page::Scaling}
					fallback={move || view! {
						<Link
							on_click={
								Rc::new(move |_| {
									let deployment_info = deployment_info.get();
									match page.get() {
										Page::Details => {
											details_error.set(DetailsPageError::new());
											if deployment_info.name.is_none() {
												details_error.update(
													|errors| errors.name = "Name is Required!".to_string()
												);
												return;
											}
											if deployment_info.registry_name.is_none() {
												details_error.update(
													|errors| errors.registry = "".to_string()
												);
												return;
											}
											if deployment_info.image_name.is_none() {
												details_error.update(
													|errors| errors.image_name = "Image Name is Required".to_string()
												);
												return;
											}
											if deployment_info.image_tag.is_none() {
												details_error.update(
													|errors| errors.image_tag = "Image Tag is also Required!".to_string()
												);

												return;
											}
											if !deployment_info.runner_id.is_some_and(|x| !x.is_empty()) {
												details_error.update(
													|errors| errors.runner = "Please Select a Runner".to_string()
												);
												return;
											}
										}
										Page::Running => {
											if deployment_info.ports.len() < 1 {
												return;
											}
										}
										Page::Scaling => {}
									};
									page.update(|x| *x = x.next());
								})
							}
							style_variant={LinkStyleVariant::Contained}
							r#type={Variant::Button}
						>
							"NEXT"
							<Icon
								icon={IconType::ChevronRight}
								size={Size::ExtraSmall}
								color={Color::Black}
								class="ml-xxs"
							/>
						</Link>
					}}
				>
					<button
						type="submit"
						class="fr-ct-ct btn btn-primary"
						on:click={on_submit}
					>
						"CREATE"
					</button>
				</Show>
			</div>
		</ContainerBody>
	}
}
