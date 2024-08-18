mod head;

use leptos_query::QueryResult;

pub use self::head::*;
use crate::{prelude::*, queries::get_runner_query};

/// The Route Params for the manage runner page
#[derive(Params, PartialEq)]
pub struct ManageRunnerRouteParams {
	runner_id: Option<String>,
}

/// The content of the manage runner page
#[component]
fn ManageRunnerContent(
	/// The runner id
	#[prop(into)]
	runner_id: Signal<Uuid>,
) -> impl IntoView {
	let QueryResult {
		data: runner_info, ..
	} = get_runner_query().use_query(move || runner_id.get());

	view! {
		<Transition>
			{
				move || {
					match runner_info.get() {
						Some(Ok(runner_info)) => view! {
							<RunnerManageHead
								runner_info={runner_info.runner.clone()}
							/>

							<ContainerBody class="p-xs px-md gap-md overflow-y-auto text-white">
								<form
									class="w-full h-full px-md py-xl flex flex-col items-start justify-start fit-wide-screen mx-auto gap-md"
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
												value={runner_info.runner.name.clone()}
											/>
										</div>
									</div>
								</form>
							</ContainerBody>
						}
						.into_view(),
						Some(Err(err)) => view! {
							<div>"Error fetching runner info"</div>
						}.into_view(),
						None => view! {
							<div>"Runner not found"</div>
						}
						.into_view(),
					}
				}
			}
		</Transition>

	}
}

/// A Wrapper around the manage runner page content to make sure that
/// the runner id is a valid Uuid
#[component]
pub fn ManageRunner() -> impl IntoView {
	let params = use_params::<ManageRunnerRouteParams>();
	let runner_id = Signal::derive(move || {
		params.with(|params| {
			let x = params
				.as_ref()
				.map(|param| param.runner_id.clone())
				.unwrap_or_default()
				.map(|x| Uuid::parse_str(x.as_str()).ok())
				.flatten();

			x
		})
	});

	move || match runner_id.get() {
		Some(runner_id) => view! {
			<ManageRunnerContent
				runner_id={Signal::derive(move || runner_id)}
			/>
		}
		.into_view(),
		None => view! {
			<div>"invalid runner id"</div>
		}
		.into_view(),
	}
}
