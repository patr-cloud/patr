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
		<ContainerBody class="p-xs px-md gap-md ofy-auto txt-white">
			<ActionForm
				action={create_runner_action}
				class="full-width full-height px-md py-xl fc-sb-fs fit-wide-screen mx-auto gap-md"
			>
				<div class="flex full-width">
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

					<div class="flex-col-2 fr-fs-fs pt-sm">
						<label html_for="name" class="txt-white txt-sm">
							"Runner Name"
						</label>
					</div>

					<div class="flex-col-10 fc-fs-fs">
						<Input
							id="name"
							name="name"
							r#type={InputType::Text}
							placeholder="Enter runner name"
							class="full-width"
						/>
					</div>
				</div>

				<div class="fr-fe-ct gap-md full-width">
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
