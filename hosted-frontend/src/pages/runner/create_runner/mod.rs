mod head;

use utils::FromToStringCodec;

pub use self::head::*;
use crate::prelude::*;

#[component]
pub fn CreateRunner() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let create_runner_action = create_server_action::<CreateRunnerFn>();
	let response = create_runner_action.value();

	view! {
		<RunnerCreateHead />
		<ContainerBody class="p-xs px-md gap-md overflow-y-auto text-white">
			<ActionForm
				action={create_runner_action}
				class="w-full h-full flex flex-col justify-between items-start px-md py-xl fit-wide-screen mx-auto gap-md"
			>
				<div class="flex w-full">
					<input
						type="hidden"
						id="access_token"
						name="access_token"
						value={move || access_token.get()}
					/>

					<input
						type="hidden"
						id="workspace_id"
						name="workspace_id"
						value={move || workspace_id.get()}
					/>

					<div class="flex-2 flex items-start justify-start pt-sm">
						<label html_for="name" class="text-white text-sm">
							"Runner Name"
						</label>
					</div>

					<div class="flex-10 flex flex-col items-start justify-start">
						<Input
							id="name"
							name="name"
							r#type={InputType::Text}
							placeholder="Enter runner name"
							class="w-full"
						/>
					</div>
				</div>

				<div class="flex items-center justify-end gap-md w-full">
					<Link
						to="/runners"
						style_variant={LinkStyleVariant::Plain}
						should_submit={false}
					>
						"Back"
					</Link>
					<Link
						style_variant={LinkStyleVariant::Contained}
						should_submit={true}
					>
						"CREATE"
					</Link>
				</div>
			</ActionForm>
		</ContainerBody>
	}
}
