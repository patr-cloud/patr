use crate::prelude::*;

#[component]
pub fn DeploymentDashboardHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="flex justify-between items-center w-full">
				<div class="flex flex-col items-start justify-start">
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

				<Link
					r#type={Variant::Link}
					to={"create".to_string()}
					style_variant={LinkStyleVariant::Contained}
				>
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
	}
}
