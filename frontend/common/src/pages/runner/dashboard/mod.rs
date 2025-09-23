mod head;
use self::head::*;
use crate::prelude::*;
/// The runner Dashboard Page
#[component]
pub fn RunnerDashboard() -> impl IntoView {
	// let runner_list = list_runners_query();
	let runner_list_headers = vec!["Name".to_string(), "Connected".to_string()];
	let runner_list = vec![
		vec!["test-runner-1".to_string(), "Disconnected".to_string()],
		vec!["test-runner-2".to_string(), "Connected".to_string()],
		vec!["test-runner-3".to_string(), "Disconnected".to_string()],
	];

	view! {
		<RunnerPage>
			<RunnerDashboardHead />
				<Table headers=runner_list_headers data=runner_list/>

		</RunnerPage>
	}
}
