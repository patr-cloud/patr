use std::rc::Rc;

use crate::prelude::*;

#[component]
pub fn ApiTokensTab() -> impl IntoView {
	let data = create_rw_signal(vec![UserApiToken {
		name: "test-token".to_string(),
		expiry: "No Expiry".to_string(),
		created: "3 Days Ago".to_string(),
	}]);

	view! {
		<div class="fc-fs-fs full-width full-height px-md py-xl gap-md">
			<TableDashboard
				column_grids=vec![4, 4, 4]
				headings=vec![
					"Name".into_view(),
					"Expiry".into_view(),
					"Created At".into_view(),
				]
				render_rows=view! {
					<For
						each=move || data.get()
						key=|state| state.name.clone()
						let:child
					>
						<ApiTokenCard
							token=child
						/>
					</For>
				}.into_view()
			/>
		</div>
	}
}

mod api_token_card;
mod edit_token;

pub use self::{api_token_card::*, edit_token::*};
