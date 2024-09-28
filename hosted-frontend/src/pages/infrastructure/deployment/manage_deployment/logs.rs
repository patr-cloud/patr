use leptos_query::QueryResult;

use super::{super::components::*, DeploymentInfoContext};
use crate::{
	prelude::*,
	queries::{get_deployment_logs_query, GetDeploymentLogsTag},
};

/// List Logs for a deployment
#[component]
pub fn ManageDeploymentsLogs() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	let QueryResult {
		data: deployment_logs,
		..
	} = get_deployment_logs_query()
		.use_query(move || GetDeploymentLogsTag(deployment_info.get().unwrap().deployment.id));

	view! {
		<div class="w-full h-full px-xl my-xl overflow-hidden">
			<div class="w-full h-full px-md flex flex-col items-start justify-start">
				<div class="w-full h-full br-sm bg-secondary px-xl py-md flex flex-col items-start justify-start overflow-auto">
					<div class="w-full pb-xxs flex justify-between items-center mb-xs gap-xl">
						<Link>"LOAD MORE"</Link>
						<p class="text-grey text-xss">"Displaying logs since {logsSince}"</p>
					</div>

					{move || match deployment_logs.get() {
						Some(Ok(deployment_logs)) => {
							view! {
								<For
									each={move || deployment_logs.logs.clone()}
									key={|state| state.timestamp}
									let:log
								>
									<LogStatement
										class="mb-xs"
										log={Signal::derive(move || log.clone())}
									/>
								</For>
							}
						}
						Some(Err(_)) => view! { "error loading logs" }.into_view(),
						None => view! { "loading logs..." }.into_view(),
					}}

				// <LogStatement log={log} class="mb-xs"/>
				</div>
			</div>
		</div>
	}
}
