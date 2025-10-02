use models::frontend::workspace::*;

use crate::prelude::*;

#[server(CreateWorkspaceFn, endpoint = "/api/workspace/create")]
pub async fn create_workspace_fn(name: String) -> Result<(), ServerFnError<ErrorType>> {
	// Here you would add the logic to create a workspace, e.g., database insertion
	Ok(())
}

#[expect(non_snake_case)]
pub fn CreateWorkspacePage((): (), CreateWorkspaceRoute {}: CreateWorkspaceRoute) -> impl IntoView {
	let create_workspace_action = ServerAction::<CreateWorkspaceFn>::new();

	view! {
		<div class="flex flex-col items-center justify-center h-full gap-4 p-4">
			<h1 class="text-2xl font-bold">"Create a New Workspace"</h1>
			<ActionForm action={create_workspace_action} attr:class="flex flex-col gap-4 w-full max-w-md">
				<label>
					"Workspace Name"
					<Input
						r#type={InputType::Text}
						name="name"
						placeholder="Workspace Name"
						class="p-2 border border-gray-300 rounded"
					/>
				</label>

				<textarea
					placeholder="Workspace Description"
					class="p-2 border border-gray-300 rounded"
				></textarea>
				// TODO: change LinkStyleVariant to ButtonVariant
				<Button variant={LinkStyleVariant::Contained}>
					"Create Workspace"
				</Button>
			</ActionForm>
		</div>
	}
}
