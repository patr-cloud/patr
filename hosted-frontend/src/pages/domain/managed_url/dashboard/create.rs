use std::{rc::Rc, str::FromStr};

use convert_case::{self, Case, Casing};
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
	let url_type = create_rw_signal("".to_string());
	let sub_domain = create_rw_signal("".to_string());
	let domain = create_rw_signal("".to_string());
	let path = create_rw_signal("".to_string());

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

				// <ManageURLForm
				// 	path={path}
				// 	domain={domain}
				// 	url_type={url_type}
				// 	is_create_mode={true}
				// 	sub_domain={sub_domain}
				// 	show_form={show_create}
				// />
			</div>
		</Modal>
	}
}
