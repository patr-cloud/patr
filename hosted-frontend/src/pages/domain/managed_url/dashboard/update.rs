use std::{rc::Rc, str::FromStr};

use convert_case::{self, Case, Casing};
use ev::SubmitEvent;
use models::api::workspace::managed_url::*;
use strum::VariantNames;

use crate::{pages::ManagedUrlCard, prelude::*};

#[component]
pub fn ManageURLForm(
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Is Create Mode or Update Mode, True is Create Mode
	#[prop(into)]
	is_create_mode: MaybeSignal<bool>,
	/// Toggle Modal Signal if in Create Mode
	#[prop(into)]
	show_form: RwSignal<bool>,
	/// Url Type of the Managed URL
	#[prop(into)]
	url_type: RwSignal<String>,
	/// Sub Domain of the Managed URL
	#[prop(into)]
	sub_domain: RwSignal<String>,
	/// Domain of the Managed URL
	#[prop(into)]
	domain: RwSignal<String>,
	/// Path of the Managed URL
	#[prop(into)]
	path: RwSignal<String>,
	/// The Port for Deployment Managed URL Type
	#[prop(into, optional, default = 0.into())]
	port: RwSignal<u16>,
	/// HTTP Only boolean for Proxy URL and Redirect
	#[prop(into, optional, default = false.into())]
	http_only: RwSignal<bool>,
	/// Permanent Redirect boolean for Redirect Managed URL Type
	#[prop(into, optional, default = false.into())]
	permanent_redirect: RwSignal<bool>,
	/// Url of the Managed URL
	#[prop(into)]
	url: RwSignal<String>,
	/// On Submit
	#[prop(into, default = Callback::new(|_| ()))]
	on_submit: Callback<SubmitEvent>,
) -> impl IntoView {
	let class = move || {
		class.with(|cname| {
			format!(
				"w-full flex flex-col justify-start items-start bg-secondary-light text-white px-xl py-md {}",
				cname
			)
		})
	};

	let port_string = create_rw_signal(format!("{:?}", port.get_untracked()));

	let domains = create_resource(
		move || {
			(
				Some("access_token".to_string()),
				Some("workspace_id".to_string()),
			)
		},
		move |(access_token, workspace_id)| async move {
			list_domains(access_token, workspace_id).await
		},
	);

	let domains_dropdown_options = Signal::derive(move || match domains.get() {
		Some(Ok(domains)) => domains
			.domains
			.iter()
			.map(|domain| InputDropdownOption {
				disabled: false,
				id: domain.id.to_string(),
				label: domain.data.domain.name.clone(),
			})
			.collect::<Vec<_>>(),
		_ => {
			vec![InputDropdownOption {
				label: "Unable To Load".to_string(),
				disabled: true,
				id: "error".to_string(),
			}]
		}
	});

	view! {
		<form
			on:submit={move |ev| {
				ev.prevent_default();
				on_submit.call(ev);
			}}
			class="flex flex-col gap-md items-center justify-start w-full"
		>
			<div class="flex items-start justify-start gap-md w-full">
				<div class="flex-3">
					<Input
						variant={SecondaryColorVariant::Medium}
						r#type={InputType::Text}
						placeholder="Add Sub Domain"
						on_input={Box::new(move |ev| {
							sub_domain.set(event_target_value(&ev));
						})}
						value={sub_domain}
						disabled={Signal::derive(move || !is_create_mode.get())}
					/>
				</div>
				<div class="flex-6">
					<Transition>
						<InputDropdown
							value={domain}
							disabled={Signal::derive(move || !is_create_mode.get())}
							variant={SecondaryColorVariant::Medium}
							placeholder={"Select Domain".to_string()}
							options={domains_dropdown_options}
							on_select={move |val: String| {
								domain.set(val);
							}}
						/>
					</Transition>
				</div>
				<div class="flex-3">
					<Input
						variant={SecondaryColorVariant::Medium}
						start_text={Some("/".to_string())}
						r#type={InputType::Text}
						value={path}
						on_input={Box::new(move |ev| {
							path.set(event_target_value(&ev));
						})}
					/>
				</div>
			</div>
			<div class="flex items-start justify-start gap-md w-full">
				<div class="flex-3">
					<InputDropdown
						value={url_type}
						on_select={move |variant: String| {
							url_type.set(variant.clone())
						}}
						variant={SecondaryColorVariant::Medium}
						placeholder={"Type".to_string()}
						options={
							ManagedUrlType::VARIANTS
								.iter()
								.map(|variant| {
									let label = variant.to_case(Case::Title);
									let id = variant.to_case(Case::Snake);

									InputDropdownOption {
										id,
										label,
										disabled: false,
									}
								})
								.collect::<Vec<_>>()
						}
					/>
				</div>
				<div class={move || url_type.with(|url_type| match ManagedUrlTypeDiscriminant::from_str(url_type) {
					Ok(ManagedUrlTypeDiscriminant::ProxyDeployment) => "flex-7",
					Ok(ManagedUrlTypeDiscriminant::ProxyStaticSite) => "flex-9",
					Ok(ManagedUrlTypeDiscriminant::Redirect) => "flex-4",
					Ok(ManagedUrlTypeDiscriminant::ProxyUrl) => "flex-7",
					_ => "flex-9"
				})}>
					<Input
						variant={SecondaryColorVariant::Medium}
						r#type={InputType::Text}
						start_text={Some("https://".to_string())}
						placeholder="URL"
						value={url}
						on_input={Box::new(move |ev| {
							url.set(event_target_value(&ev));
						})}
					/>
				</div>
				{
					move || url_type.with(|url_type| match ManagedUrlTypeDiscriminant::from_str(url_type) {
						Ok(ManagedUrlTypeDiscriminant::ProxyDeployment) => view! {
							<div class="flex-2">
								<InputDropdown
									placeholder="Select Port"
									variant={SecondaryColorVariant::Medium}
									value={port_string}
									on_select={move |val: String| {
										if let Ok(val) = u16::from_str(val.as_str()) {
											port.set(val);
										}
									}}
									options={vec![
										InputDropdownOption {
											id: "3000".to_string(),
											label: "3000".to_string(),
											disabled: false
										}
									]}
								/>
							</div>
						}.into_view(),
						Ok(ManagedUrlTypeDiscriminant::ProxyStaticSite) => view! {
							<div class="flex-2">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input
										type="checkbox"
										prop:checked={move || http_only.get()}
										on:input={move |_| {
											http_only.update(|v| *v = !*v);
										}}
									/>
									<p>"HTTP ONLY"</p>
								</label>
							</div>
						}.into_view(),
						Ok(ManagedUrlTypeDiscriminant::Redirect) => view! {
							<div class="flex-2">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input
										type="checkbox"
										prop:checked={move || http_only.get()}
										on:input={move |_| {
											http_only.update(|v| *v = !*v);
										}}
									/>
									<p>"HTTP ONLY"</p>
								</label>
							</div>

							<div class="flex-3">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input
										prop:checked={permanent_redirect}
										type="checkbox"
										on:input={move |_| {
											permanent_redirect.update(|val| *val = !*val)
										}}
									/>
									<p>"PERMANENT REDIRECT"</p>
								</label>
							</div>
						}.into_view(),
						Ok(ManagedUrlTypeDiscriminant::ProxyUrl) => view! {
							<div class="flex-2">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input
										type="checkbox"
										prop:checked={http_only}
										on:input={move |_| {
											http_only.update(|val| *val = !*val)
										}}
									/>
									<p>"HTTP ONLY"</p>
								</label>
							</div>
						}.into_view(),
						_ => view! { }.into_view()
					})
				}
			</div>
			<div class="w-full flex justify-end items-center mt-auto gap-md">
				<Link
					should_submit=false
					class="text-white"
					on_click={Rc::new(move |_| show_form.set(false))}
				>
					"CANCEL"
				</Link>

				<Show
					when={move || is_create_mode.get()}
					fallback={move || view! {
						<Link
							should_submit={true}
							style_variant={LinkStyleVariant::Contained}
						>
							"UPDATE"
						</Link>
					}}
				>
					<Link
						should_submit={true}
						style_variant={LinkStyleVariant::Contained}
					>
						"CREATE"
					</Link>
				</Show>
			</div>
		</form>
	}
}
