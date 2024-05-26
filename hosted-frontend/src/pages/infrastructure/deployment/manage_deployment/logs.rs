use crate::prelude::*;

#[component]
pub fn ManageDeploymentsLogs() -> impl IntoView {
	let log = LogItem {
		timestamp: "12:00".to_owned(),
		log: "npm run build".to_owned(),
	};

	view! {
		<div class="full-width full-height px-xl my-xl of-hidden">
			<div class="full-width full-height px-md fc-fs-fs">
				<div class="full-width full-height br-sm bg-secondary px-xl py-md fc-fs-fs of-auto">
					<div class="full-width pb-xxs fr-sb-ct ul-light mb-xs gap-xl">
						<Link>"LOAD MORE"</Link>
						<p class="txt-grey txt-xss">"Displaying logs since {logsSince}"</p>
					</div>

					<LogStatement log={log} class="mb-xs"/>
				</div>
			</div>
		</div>
	}
}
