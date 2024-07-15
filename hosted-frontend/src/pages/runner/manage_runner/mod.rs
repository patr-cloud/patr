mod head;

use utils::FromToStringCodec;

pub use self::head::*;
use crate::prelude::*;

#[derive(Params, PartialEq)]
pub struct ManageRunnerRouteParams {
	runner_id: Option<String>,
}

#[component]
pub fn ManageRunner() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>("accessToken");
	let (workspace_id, _) = use_cookie::<String, FromToStringCodec>("lastUsedWorkspaceId");

	let params = use_params::<ManageRunnerRouteParams>();
	let runner_id = Signal::derive(move || {
		params.with(|params| {
			params
				.as_ref()
				.map(|param| param.runner_id.clone())
				.unwrap_or_default()
		})
	});
	let runner_info = create_resource(
		move || (access_token.get(), runner_id.get(), workspace_id.get()),
		move |(access_token, runner_id, workspace_id)| async move {
			get_runner(access_token, runner_id, workspace_id).await
		},
	);

	view! {
		<Transition>
			{
				move || match runner_info.get() {
					Some(info) => {
						match info {
							Ok(runner_info) => {
								view! {
									<RunnerManageHead
										runner_info={runner_info.runner.clone()}
									/>
									<ContainerBody class="p-xs px-md gap-md ofy-auto txt-white">
										<form
											class="full-width full-height px-md py-xl fc-fs-fs
												fit-wide-screen mx-auto gap-md"
										>
											<div class="flex full-width">
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
														value={runner_info.runner.name.clone()}
													/>
												</div>
											</div>
										</form>
									</ContainerBody>
								}.into_view()
							},
							Err(err) => {
								logging::log!("{:?}", err);
								view! {
									<div>"Error Fetching Data"</div>
								}.into_view()
							},
						}
					},
					None => view! {
						<div>"Loading"</div>
					}.into_view()
				}
			}
		</Transition>
	}
}
