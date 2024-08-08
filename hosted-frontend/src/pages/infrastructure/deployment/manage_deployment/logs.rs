use crate::prelude::*;

#[component]
pub fn ManageDeploymentsLogs() -> impl IntoView {
	let log = LogItem {
		timestamp: "12:00".to_owned(),
		log: "npm run build".to_owned(),
	};

	view! {
		<div class="w-full h-full px-xl my-xl overflow-hidden">
			<div class="w-full h-full px-md flex flex-col items-start justify-start">
				<div class="w-full h-full br-sm bg-secondary px-xl py-md flex flex-col items-start justify-start overflow-auto">
					<div class="w-full pb-xxs flex justify-between items-center border-2 border-border-color mb-xs gap-xl">
						<Link>"LOAD MORE"</Link>
						<p class="text-grey text-xss">"Displaying logs since {logsSince}"</p>
					</div>

					<LogStatement log={log} class="mb-xs"/>
				</div>
			</div>
		</div>
	}
}
