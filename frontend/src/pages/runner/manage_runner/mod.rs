mod head;

use convert_case::*;

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
	let runner_info = get_runner_query(runner_id);

	view! {
		<Transition>
			{move || {
				match runner_info.get() {
					Some(Ok(runner_info)) => {
						view! {
							<RunnerManageHead runner_info={runner_info.runner.clone()} />

							<ContainerBody class="p-xs px-md gap-md overflow-y-auto text-white">
								<form class="w-full h-full px-md py-xl flex flex-col items-start justify-start fit-wide-screen mx-auto gap-md">
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
							.into_view()
					}
					Some(Err(err)) => {
						view! {
							<div>"Error fetching runner info"</div>
							<ErrorPage
								title="Error Loading Runner Info"
								content={view! {
									<p class="text-white">
										{format!("{}", err.to_string().to_case(Case::Title))}
									</p>
								}
									.into_view()}
							/>
						}
							.into_view()
					}
					None => view! { <div>"Loading Runner Info"</div> }.into_view(),
				}
			}}
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
			params
				.as_ref()
				.map(|param| param.runner_id.clone())
				.unwrap_or_default()
				.map(|x| Uuid::parse_str(x.as_str()).ok())
				.flatten()
		})
	});

	move || match runner_id.get() {
		Some(runner_id) => {
			view! { <ManageRunnerContent runner_id={Signal::derive(move || runner_id)} /> }
				.into_view()
		}
		None => view! {
			<ErrorPage
				title="Invalid Runner ID"
				content={view! { <p class="text-white">"Runner id is not a valid UUID"</p> }
					.into_view()}
			/>
		}
		.into_view(),
	}
}
