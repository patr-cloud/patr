use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::prelude::*;

#[component]
pub fn CreateWorkspace() -> impl IntoView {
	let create_workspace_action = create_server_action::<CreateWorkspaceFn>();
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

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
			<ActionForm
				action={create_workspace_action}
				class="full-width full-height gap-md fc-fs-fs px-md fit-wide-screen"
			>
				<input type="hidden" name="access_token" prop:value={access_token} />
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
							id="workspace_name"
							name="workspace_name"
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
			</ActionForm>
		</ContainerBody>
	}
}
