use crate::{pages::*, prelude::*};

#[component]
pub fn ManageDeploymentImageHistory() -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width px-xl my-sm mx-auto fit-wide-screen">
			<div class="fc-fs-fs full-width full-height">
				<div class="full-width ul-light mb-xl pb-xs">
					// <div style="width: 100%; height: 3rem;"></div>
					<ImageHistoryCard active=true />
				</div>

				<ImageHistoryCard active=false />
				<ImageHistoryCard active=false />
			</div>
		</div>
	}
}
