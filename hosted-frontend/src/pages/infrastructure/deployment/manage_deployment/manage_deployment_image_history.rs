use crate::{pages::*, prelude::*};

#[component]
pub fn ManageDeploymentImageHistory() -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width px-md my-sm mx-auto fit-wide-screen">
			<div class="fc-fs-fs full-width full-height gap-sm">
				<div class="full-width ul-light mb-md pb-xs">
					// <div style="width: 100%; height: 3rem;"></div>
					<ImageHistoryCard active=true />
				</div>

				<ImageHistoryCard active=false />
				<ImageHistoryCard active=false />
			</div>
		</div>
	}
}
