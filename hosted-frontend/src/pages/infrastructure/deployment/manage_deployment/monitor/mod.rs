mod event_log_card;
mod event_log_container;

use self::{event_log_card::*, event_log_container::*};
use crate::prelude::*;

#[component]
pub fn ManageDeploymentsMonitoring() -> impl IntoView {
	view! {
		<div class="full-width full-height px-xl my-xl of-hidden">
			<EventLogContainer />
		</div>
	}
}
