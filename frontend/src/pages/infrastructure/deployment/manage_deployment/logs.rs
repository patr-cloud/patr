use std::rc::Rc;

use ev::MouseEvent;
use models::api::workspace::deployment::DeploymentLog;
use time::{macros::format_description, Duration, OffsetDateTime};

use super::{super::components::*, DeploymentInfoContext};
use crate::prelude::*;

/// List Logs for a deployment
#[component]
pub fn ManageDeploymentsLogs() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;
	let (state, _) = AuthState::load();

	let logs_list = create_rw_signal::<Vec<DeploymentLog>>(vec![]);

	let end_time = create_rw_signal(OffsetDateTime::now_utc());
	let deployment_logs = create_resource(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
				end_time.get(),
			)
		},
		move |(access_token, workspace_id, end_time)| async move {
			get_deployment_logs(
				access_token,
				workspace_id,
				deployment_info.get().unwrap().deployment.id,
				Some(end_time),
				Some(25),
			)
			.await
		},
	);

	create_effect(move |_| match deployment_logs.get() {
		Some(Ok(new_logs)) => logs_list.update(|logs| {
			logs.extend(new_logs.logs.into_iter());
			// REMOVE THIS FROM HERE
			logs.extend((0..25).map(|x| DeploymentLog {
				timestamp: end_time.get() - Duration::seconds(x * 100),
				log: format!("This is a log {x}"),
			}))
			// TO HERE
		}),
		_ => {}
	});

	let on_click_load = move |_: &MouseEvent| {
		let earliest_timestamp = logs_list.get().last().map(|log| log.timestamp);

		if let Some(earliest_timestamp) = earliest_timestamp {
			end_time.set(earliest_timestamp);
		}
	};

	let date_formater = format_description!("[year]-[month]-[day] [hour]:[minute]");

	view! {
		<div class="w-full h-full px-xl my-xl overflow-hidden">
			<div class="w-full h-full px-md flex flex-col items-start justify-start">
				{
					move || match deployment_logs.get() {
						Some(Ok(deployment_logs)) => {
							view! {
								<div class="w-full pb-xxs flex justify-between items-center mb-xs gap-xl">
									<Link
										on_click={Rc::new(on_click_load)}
									>
										"LOAD MORE"
									</Link>
								</div>
								<div class="w-full h-full br-sm bg-secondary px-xl py-md flex flex-col items-start justify-start overflow-auto">
									<For
										each={move || logs_list.get()}
										key={|state| state.timestamp}
										let:log
									>
										<LogStatement
											class="mb-xs"
											log={
												Signal::derive(move || log.clone())
											}
										/>
									</For>
								</div>
							}.into_view()
						}
						Some(Err(_)) => view! { "error loading logs" }.into_view(),
						None => view! { "loading logs..." }.into_view(),
					}
				}
			</div>
		</div>
	}
}
