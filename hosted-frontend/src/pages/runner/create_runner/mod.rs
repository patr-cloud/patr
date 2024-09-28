mod head;

pub use self::head::*;
use crate::{prelude::*, queries::create_runner_query};

/// The Create Runner Page
#[component]
pub fn CreateRunner() -> impl IntoView {
	let runner_name = create_rw_signal("".to_string());

	let create_runner_action = create_runner_query();

	view! {
		<RunnerCreateHead />
		<ContainerBody class="p-xs px-md gap-md overflow-y-auto text-white">
			<form
				on:submit={move |ev| {
					ev.prevent_default();
					create_runner_action.dispatch(runner_name.get());
				}}
				class="w-full h-full flex flex-col justify-between items-start px-md py-xl fit-wide-screen mx-auto gap-md"
			>
				<div class="flex w-full">
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
							value={runner_name}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								runner_name.set(event_target_value(&ev))
							})}
						/>
					</div>
				</div>

				<div class="flex items-center justify-end gap-md w-full">
					<Link to="/runners" style_variant={LinkStyleVariant::Plain} should_submit=false>
						"Back"
					</Link>
					<Link style_variant={LinkStyleVariant::Contained} should_submit=true>
						"CREATE"
					</Link>
				</div>
			</form>
		</ContainerBody>
	}
}
