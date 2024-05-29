use crate::prelude::*;

#[component]
pub fn CreateWorkspace() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fc-fs-fs">
				<PageTitleContainer>
					<PageTitle to="/workspace">"Workspace"</PageTitle>
					<PageTitle
						variant=PageTitleVariant::SubHeading
						icon_position=PageTitleIconPosition::Start
					>
						"Create Workspace"
					</PageTitle>
				</PageTitleContainer>
				<PageDescription description="Create a new workspace here." />
			</div>
		</ContainerHead>

		<ContainerBody class="px-xl py-lg ofy-auto txt-white">
			<form
				class="full-width full-height gap-md fc-fs-fs px-md fit-wide-screen"
			>
				<div class="flex full-width">
					<div class="flex-col-2 pt-sm">
						<label html_for="name" class="txt-sm">
							"Workspace Name"
						</label>
					</div>
					<div class="flex-col-10 fc-fs-fs gap-xxs">
						<Input
							placeholder="Enter workspace name"
							class="full-width"
						/>
						// {workspaceNameError && (
						//     <Alert message={workspaceNameError} type="error" />
						// )}
					</div>
				</div>

				<div class="fr-fs-ct gap-md ml-auto mt-auto">
					<Link
						class="txt-sm txt-medium"
						r#type={Variant::Link}
						style_variant={LinkStyleVariant::Plain}
					>
						"BACK"
					</Link>
					<Link
						should_submit={true}
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Contained}
					>
						"CREATE"
					</Link>
				</div>
			</form>
		</ContainerBody>
	}
}
