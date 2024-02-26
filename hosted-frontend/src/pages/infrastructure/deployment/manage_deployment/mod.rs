use crate::prelude::*;

#[component]
pub fn ManageDeployments() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle icon_position=PageTitleIconPosition::End>
								"Infrastructure"
							</PageTitle>
							<PageTitle
								icon_position=PageTitleIconPosition::End
								variant=PageTitleVariant::SubHeading
							>
								"Deployment"
							</PageTitle>
							<PageTitle variant=PageTitleVariant::Text>
								"Deployment Name"
							</PageTitle>
						</PageTitleContainer>
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

				<Tabs
					tab_items=vec![
						TabItem {
							name: "Details".to_owned(),
							path: "/somepath".to_owned()
						},
						TabItem {
							name: "Scaling".to_owned(),
							path: "/somepath".to_owned()
						},
						TabItem {
							name: "URLs".to_owned(),
							path: "/somepath".to_owned()
						},
						TabItem {
							name: "Image History".to_owned(),
							path: "/somepath".to_owned()
						},
						TabItem {
							name: "Logs".to_owned(),
							path: "/somepath".to_owned()
						},
					]
				/>
			</ContainerHead>

			<ContainerBody class="gap-md">
				<ManageDeploymentsLogs />
			</ContainerBody>
		</ContainerMain>
	}
}

mod image_history_card;
mod image_tag;
mod logs;
mod manage_deployment_details;
mod manage_deployment_image_history;
mod manage_deployment_scaling;
mod manage_deployment_urls;

pub use self::{
	image_history_card::*,
	image_tag::*,
	logs::*,
	manage_deployment_details::*,
	manage_deployment_image_history::*,
	manage_deployment_scaling::*,
	manage_deployment_urls::*,
};
