use crate::{imports::*, pages::DeploymentCard};

/// TO BE REPLACED LATER WITH MODEL A PROPER MODEL TYPE
/// ACCORDING TO THE REQUEST RESPONSE TYPE
#[derive(PartialEq, Eq, Clone)]
pub struct DeploymentType {
	pub id: String,
	pub name: String,
	pub image_tag: String,
	pub status: Status,
	pub region: String,
	pub machine_type: String,
}

#[component]
pub fn DeploymentDashboard() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle icon_position=PageTitleIconPosition::End>
								"Infrastructure"
							</PageTitle>
							<PageTitle variant=PageTitleVariant::SubHeading>
								"Infrastructure"
							</PageTitle>
						</PageTitleContainer>

						<PageDescription
							description="Create and Manage Deployments with ease using Patr."
							doc_link=Some("https://docs.patr.cloud/features/deployments/".to_owned())
						/>
					</div>

					<Link r#type=Variant::Button style_variant=LinkStyleVariant::Contained>
						"CREATE DEPLOYMENT"
						<Icon
							icon=IconType::Plus
							size=Size::ExtraSmall
							class="ml-xs"
							color=Color::Black
						/>
					</Link>
				</div>
			</ContainerHead>

			<ContainerBody>
				<section class="p-xl full-width ofy-auto">
					/// TODO CHANGE THIS TO THE GRID CLASSES
					<div style="display:grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; align-content: start; justify-content: start" class="">
						/// CHANGE THIS TO A <For /> Component
						<DeploymentCard deployment=DeploymentType {
							id: "53184348".to_owned(),
							name: "Depoymentl".to_owned(),
							image_tag: "asdqwdadawdasdasd".to_owned(),
							status: Status::Created,
							region: "North America".to_owned(),
							machine_type: "1vCPU 0.5GB".to_owned(),
						} />
						<DeploymentCard deployment=DeploymentType {
							id: "784654685".to_owned(),
							name: "Depoymentl".to_owned(),
							image_tag: "asdqwdafwedwddwdqwd".to_owned(),
							status: Status::Deploying,
							region: "China".to_owned(),
							machine_type: "1vCPU 0.5GB".to_owned(),
						} />
						<DeploymentCard deployment=DeploymentType {
							id: "12343123".to_owned(),
							name: "Depoymentl".to_owned(),
							image_tag: "ejlkjfweieq".to_owned(),
							status: Status::Live,
							region: "APAC".to_owned(),
							machine_type: "1vCPU 0.5GB".to_owned(),
						} />
						<DeploymentCard deployment=DeploymentType {
							id: "4345398435".to_owned(),
							name: "Depoymentl".to_owned(),
							image_tag: "asdqwdsawasda".to_owned(),
							status: Status::Stopped,
							region: "APAC".to_owned(),
							machine_type: "1vCPU 0.5GB".to_owned(),
						} />
						<DeploymentCard deployment=DeploymentType {
							id: "8486546851".to_owned(),
							name: "Depoymentl".to_owned(),
							image_tag: "cfgjljijadkqwd".to_owned(),
							status: Status::Errored,
							region: "EMEA".to_owned(),
							machine_type: "1vCPU 0.5GB".to_owned(),
						} />
					</div>
				</section>
			</ContainerBody>
		</ContainerMain>
	}
}
