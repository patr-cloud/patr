use crate::prelude::*;

#[component]
pub fn CreateWorkspace() -> impl IntoView {
	let create_workspace_action = create_server_action::<CreateWorkspaceFn>();
	let (state, _) = AuthState::load();

	view! {
		<ContainerHead>
			<div class="flex flex-col items-start justify-start">
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

		<ContainerBody class="px-xl py-lg overflow-y-auto text-white">
			<ActionForm
				action={create_workspace_action}
				class="w-full h-full px-md fit-wide-screen \
				gap-md flex flex-col items-start justify-start"
			>
				<input type="hidden" name="access_token" prop:value={state.get_untracked().get_access_token()} />
				<div class="flex w-full">
					<div class="flex-2 pt-sm">
						<label html_for="name" class="text-sm">
							"Workspace Name"
						</label>
					</div>
					<div class="flex-10 flex flex-col items-start justify-start gap-xxs">
						<Input
							placeholder="Enter workspace name"
							class="w-full"
							id="workspace_name"
							name="workspace_name"
						/>
					</div>
				</div>

				<div class="flex items-center justify-start gap-md ml-auto mt-auto">
					<Link
						class="text-sm text-medium"
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
