use convert_case::*;
use ev::SubmitEvent;
use models::api::workspace::managed_url::*;

use crate::{pages::ManagedURLForm, prelude::*};

/// Component for Updating managed url
#[component]
pub fn UpdateManagedUrl(
	/// Signal to toggle the component
	#[prop(into)]
	show_component: RwSignal<bool>,
	/// Managed URL Item
	#[prop(into)]
	managed_url: Signal<WithId<ManagedUrl>>,
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	#[allow(unused)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let store_managed_url = store_value(managed_url);
	let managed_url_id = Signal::derive(move || managed_url.clone().get().id);

	let url_type = create_rw_signal(
		store_managed_url.with_value(|val| val.get_untracked().url_type.to_string()),
	);
	let sub_domain = create_rw_signal(
		store_managed_url.with_value(|val| val.get_untracked().sub_domain.clone()),
	);
	let domain = create_rw_signal(
		store_managed_url.with_value(|val| val.get_untracked().domain_id.to_string()),
	);
	let path =
		create_rw_signal(store_managed_url.with_value(|val| val.get_untracked().path.clone()));
	let url =
		create_rw_signal(
			store_managed_url.with_value(|val| match &val.get_untracked().url_type {
				ManagedUrlType::ProxyDeployment { deployment_id, .. } => deployment_id.to_string(),
				ManagedUrlType::ProxyStaticSite { static_site_id } => static_site_id.to_string(),
				ManagedUrlType::Redirect { url, .. } => url.clone(),
				ManagedUrlType::ProxyUrl { url, .. } => url.clone(),
			}),
		);
	let port = create_rw_signal::<u16>(store_managed_url.with_value(|val| {
		match val.get_untracked().url_type {
			ManagedUrlType::ProxyDeployment { port, .. } => port,
			_ => 0,
		}
	}));
	let http_only =
		create_rw_signal(
			store_managed_url.with_value(|val| match val.get_untracked().url_type {
				ManagedUrlType::Redirect { http_only, .. } => http_only,
				ManagedUrlType::ProxyUrl { http_only, .. } => http_only,
				_ => false,
			}),
		);
	let permanent_redirect =
		create_rw_signal(
			store_managed_url.with_value(|val| match val.get_untracked().url_type {
				ManagedUrlType::Redirect {
					permanent_redirect, ..
				} => permanent_redirect,
				_ => false,
			}),
		);

	let (state, _) = AuthState::load();
	let access_token = Signal::derive(move || state.get().get_access_token());
	let current_workspace_id = Signal::derive(move || state.get().get_last_used_workspace_id());

	let on_submit_update = move |_: SubmitEvent| {
		spawn_local(async move {
			logging::log!("{} {}", url.get(), url_type.get());

			let resp = update_managed_url(
				current_workspace_id
					.get_untracked()
					.map(|uuid| uuid.to_string()),
				access_token.get_untracked(),
				path.get_untracked(),
				managed_url_id.get_untracked().to_string(),
				url_type.get_untracked().to_case(Case::Camel),
				url.get_untracked(),
				port.get_untracked(),
				http_only.get_untracked(),
				permanent_redirect.get_untracked(),
			)
			.await;

			if resp.is_ok() {
				show_component.set(false);
			}
		});
	};

	view! {
		<td class="w-full flex items-center justify-center py-lg px-xl overflow-y-scroll">
			<ManagedURLForm
				url_type={url_type}
				domain={domain}
				sub_domain={sub_domain}
				path={path}
				is_create_mode=false
				show_form={show_component}
				url={url}
				port={port}
				http_only={http_only}
				permanent_redirect={permanent_redirect}
				on_submit={on_submit_update}
			/>
		</td>
	}
}
