use std::{rc::Rc, str::FromStr};

use convert_case::{Case, Casing};
use ev::SubmitEvent;
use models::api::workspace::managed_url::*;
use strum::VariantNames;

use crate::prelude::*;

#[component]
pub fn ManagedURLForm(
	/// The class names to add to the outer table row
	#[prop(into, optional)]
	#[allow(unused)]
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
	let (state, _) = AuthState::load();
	let access_token = Signal::derive(move || state.get().get_access_token());
	let current_workspace_id = Signal::derive(move || state.get().get_last_used_workspace_id());

	let port_string = create_rw_signal(format!("{:?}", port.get_untracked()));

	let deployment_list = get_deployments();

	let domains = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
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
						r#type={InputType::Text}
						value={path}
						on_input={Box::new(move |ev| {
							path.set(event_target_value(&ev));
						})}
					/>
				</div>
			</div>
			<div class="flex items-start justify-start gap-md w-full">
				{logging::log!("{:?}", url_type.get())} <div class="flex-3">
					<InputDropdown
						value={url_type.with(|val| val.to_case(Case::Snake))}
						on_select={move |variant: String| { url_type.set(variant.clone()) }}
						variant={SecondaryColorVariant::Medium}
						placeholder={"Type".to_string()}
						options={ManagedUrlType::VARIANTS
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
							.collect::<Vec<_>>()}
					/>
				</div>
				{move || {
					url_type
						.with(|url_type| match ManagedUrlTypeDiscriminant::from_str(url_type) {
							Ok(ManagedUrlTypeDiscriminant::ProxyDeployment) => {
								view! {
									<Transition>
										{move || match deployment_list.get() {
											Some(Ok(deployments)) => {
												view! {
													<div class="flex-7">
														<InputDropdown
															variant={SecondaryColorVariant::Medium}
															placeholder="Select Deployment"
															on_select={move |val: String| {
																url.set(val);
															}}
															options={deployments
																.deployments
																.iter()
																.map(|deployment| InputDropdownOption {
																	id: deployment.id.to_string(),
																	label: deployment.name.clone(),
																	disabled: false,
																})
																.collect::<Vec<_>>()}
														/>
													</div>

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
															options={vec![]}
														/>
													</div>
												}
													.into_view()
											}
											Some(Err(_)) => {
												view! {
													<div class="flex-9">
														<InputDropdown
															variant={SecondaryColorVariant::Medium}
															placeholder="Error Loading Deployments"
															options={vec![
																InputDropdownOption {
																	id: "error".to_string(),
																	label: "Error Loading Deployments".to_string(),
																	disabled: true,
																},
															]}
														/>
													</div>
												}
													.into_view()
											}
											None => view! {}.into_view(),
										}}
									</Transition>
								}
									.into_view()
							}
							Ok(ManagedUrlTypeDiscriminant::ProxyStaticSite) => {
								view! {
									<div class="flex-9">
										<InputDropdown
											variant={SecondaryColorVariant::Medium}
											placeholder="Select Static Site"
											options={vec![]}
										/>
									</div>

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
								}
									.into_view()
							}
							Ok(ManagedUrlTypeDiscriminant::Redirect) => {
								view! {
									<div class="flex-4">
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
								}
									.into_view()
							}
							Ok(ManagedUrlTypeDiscriminant::ProxyUrl) => {
								view! {
									<div class="flex-7">
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

									<div class="flex-2">
										<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
											<input
												type="checkbox"
												prop:checked={http_only}
												on:input={move |_| { http_only.update(|val| *val = !*val) }}
											/>
											<p>"HTTP ONLY"</p>
										</label>
									</div>
								}
									.into_view()
							}
							_ => {
								view! {
									<div class="flex-9">
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
								}
									.into_view()
							}
						})
				}}
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
					fallback={move || {
						view! {
							<Link should_submit=true style_variant={LinkStyleVariant::Contained}>
								"UPDATE"
							</Link>
						}
					}}
				>
					<Link should_submit=true style_variant={LinkStyleVariant::Contained}>
						"CREATE"
					</Link>
				</Show>
			</div>
		</form>
	}
}
