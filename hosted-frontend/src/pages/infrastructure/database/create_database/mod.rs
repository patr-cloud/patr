mod head;

pub use self::head::*;
use crate::prelude::*;

#[component]
pub fn CreateDatabase() -> impl IntoView {
	view! {
		<CreateDatabaseHeader />
		<div class="txt-white">"CREATE DATABASE"</div>
	}
}
