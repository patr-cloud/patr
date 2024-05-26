use crate::{pages::DeploymentCard, prelude::*};

/// Deployment Model
/// TO BE REPLACED LATER WITH MODEL A PROPER MODEL TYPE
/// ACCORDING TO THE REQUEST RESPONSE TYPE
#[derive(PartialEq, Eq, Clone)]
pub struct DeploymentType {
	/// The Id of the deployment
	pub id: String,
	/// The Name of the deployment
	pub name: String,
	/// The Image Tag of the deployment
	pub image_tag: String,
	/// The Status of the deployment
	pub status: Status,
	/// The Region of the deployment
	pub region: String,
	/// The Machine Type of the deployment
	pub machine_type: String,
}

#[component]
pub fn Deployment() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<Outlet/>
		</ContainerMain>
	}
}

#[component]
pub fn DeploymentDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![
		DeploymentType {
			id: "53184348".to_owned(),
			name: "Depoymentl".to_owned(),
			image_tag: "asdqwdadawdasdasd".to_owned(),
			status: Status::Created,
			region: "North America".to_owned(),
			machine_type: "1vCPU 0.5GB".to_owned(),
		},
		DeploymentType {
			id: "784654685".to_owned(),
			name: "Depoymentl".to_owned(),
			image_tag: "asdqwdafwedwddwdqwd".to_owned(),
			status: Status::Deploying,
			region: "China".to_owned(),
			machine_type: "1vCPU 0.5GB".to_owned(),
		},
		DeploymentType {
			id: "12343123".to_owned(),
			name: "Depoymentl".to_owned(),
			image_tag: "ejlkjfweieq".to_owned(),
			status: Status::Live,
			region: "APAC".to_owned(),
			machine_type: "1vCPU 0.5GB".to_owned(),
		},
		DeploymentType {
			id: "4345398435".to_owned(),
			name: "Depoymentl".to_owned(),
			image_tag: "asdqwdsawasda".to_owned(),
			status: Status::Stopped,
			region: "APAC".to_owned(),
			machine_type: "1vCPU 0.5GB".to_owned(),
		},
		DeploymentType {
			id: "8486546851".to_owned(),
			name: "Depoymentl".to_owned(),
			image_tag: "cfgjljijadkqwd".to_owned(),
			status: Status::Errored,
			region: "EMEA".to_owned(),
			machine_type: "1vCPU 0.5GB".to_owned(),
		},
	]);
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::SubHeading}>"Deployment"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description="Create and Manage Deployments with ease using Patr."
						doc_link={Some("https://docs.patr.cloud/features/deployments/".to_owned())}
					/>
				</div>

				<Link r#type={Variant::Button} style_variant={LinkStyleVariant::Contained}>
					"CREATE DEPLOYMENT"
					<Icon
						icon={IconType::Plus}
						size={Size::ExtraSmall}
						class="ml-xs"
						color={Color::Black}
					/>
				</Link>
			</div>
		</ContainerHead>

		<ContainerBody>
			<DashboardContainer
				gap={Size::Large}
				render_items={
					view! {
						<For each={move || data.get()} key={|state| state.id.clone()} let:child>
							<DeploymentCard deployment={child}/>
						</For>
					}
				}
			/>

		</ContainerBody>
	}
}
