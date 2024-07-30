use std::{rc::Rc, str::FromStr};

use convert_case::{self, Case, Casing};
use leptos_use::utils::FromToStringCodec;
use models::api::workspace::managed_url::*;
use strum::VariantNames;

use super::ManageURLForm;
use crate::prelude::*;

#[component]
pub fn CreateManagedUrlDashboard(
	/// Show Modal Signal
	#[prop(into)]
	show_create: RwSignal<bool>,
) -> impl IntoView {
	let url = create_rw_signal("".to_string());
	let path = create_rw_signal("".to_string());
	let domain = create_rw_signal("".to_string());
	let url_type = create_rw_signal("".to_string());
	let sub_domain = create_rw_signal("".to_string());

	let port = create_rw_signal(0);
	let http_only = create_rw_signal(false);
	let perma_redirect = create_rw_signal(false);

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let on_submit_create = move |_| {
		spawn_local(async move {
			logging::log!("submit event\n {:#?}", domain.get_untracked());
			let resp = create_managed_url(
				current_workspace_id.get_untracked(),
				access_token.get_untracked(),
				sub_domain.get_untracked(),
				domain.get_untracked(),
				path.get_untracked(),
				url_type.get_untracked(),
				url.get_untracked(),
				port.get_untracked(),
				http_only.get_untracked(),
				perma_redirect.get_untracked(),
			)
			.await;

			logging::log!("event resp {:?}", resp);
		});
	};

	view! {
		<Modal color_variant={SecondaryColorVariant::Light}>
			<div
				class="w-3/5 center-modal text-white text-sm flex flex-col items-start justify-start bg-secondary-light br-sm p-xl show-center-modal gap-lg"
			>
				<div class="flex items-center justify-start">
					<h3 class="text-white text-lg">"Create New Managed URL"</h3>
					<Link to="">
						"Documentation"
						<Icon
							icon=IconType::Link
							size=Size::ExtraExtraSmall
							color=Color::Primary
							class="ml-xxs"
						/>
					</Link>
				</div>

				<ManageURLForm
					url={url}
					path={path}
					domain={domain}
					url_type={url_type}
					is_create_mode={true}
					sub_domain={sub_domain}
					show_form={show_create}
					on_submit={on_submit_create}
				/>
			</div>
		</Modal>
	}
}
