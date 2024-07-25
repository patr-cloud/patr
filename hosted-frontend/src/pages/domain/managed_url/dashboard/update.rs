use std::{rc::Rc, str::FromStr};

use convert_case::{self, Case, Casing};
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
	// Url Type of the Managed URL
	// #[prop(into)]
	// url_type: RwSignal<String>,
	// Sub Domain of the Managed URL
	// #[prop(into)]
	// sub_domain: RwSignal<String>,
	// Domain of the Managed URL
	// #[prop(into)]
	// domain: RwSignal<String>,
	// Path of the Managed URL
	// #[prop(into)]
	// path: RwSignal<String>,
) -> impl IntoView {
	let class = move || {
		class.with(|cname| {
			format!(
				"w-full flex flex-col justify-start items-start bg-secondary-light text-white px-xl py-md {}",
				cname
			)
		})
	};

	let url_type = create_rw_signal("".to_string());

	view! {
		<form
			class="flex flex-col gap-md items-center justify-start w-full"
		>
			<div class="flex items-start justify-start gap-md w-full">
				<div class="flex-3">
					<Input
						variant={SecondaryColorVariant::Medium}
						r#type={InputType::Text}
						placeholder="Add Sub Domain"
						// value={sub_domain}
						// disabled={Signal::derive(move || !is_create_mode.get())}
					/>
				</div>
				<div class="flex-6">
					<Input
						variant={SecondaryColorVariant::Medium}
						r#type={InputType::Text}
						placeholder="Domain"
						// value={domain}
						// disabled={Signal::derive(move || !is_create_mode.get())}
					/>
				</div>
				<div class="flex-3">
					<Input
						variant={SecondaryColorVariant::Medium}
						start_text={Some("/".to_string())}
						r#type={InputType::Text}
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
				{
					move || url_type.with(|url_type| match ManagedUrlTypeDiscriminant::from_str(url_type) {
						Ok(ManagedUrlTypeDiscriminant::ProxyDeployment) => view! {
							<div class="flex-7">
								<Input
									variant={SecondaryColorVariant::Medium}
									r#type={InputType::Text}
									start_text={Some("https://".to_string())}
									placeholder="URL"
								/>
							</div>
							<div class="flex-2">
								<InputDropdown
									placeholder="Select Port"
									variant={SecondaryColorVariant::Medium}
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
							<div class="flex-9">
								<Input
									variant={SecondaryColorVariant::Medium}
									r#type={InputType::Text}
									start_text={Some("https://".to_string())}
									placeholder="URL"
								/>
							</div>
						}.into_view(),
						Ok(ManagedUrlTypeDiscriminant::Redirect) => view! {
							<div class="flex-4">
								<Input
									variant={SecondaryColorVariant::Medium}
									r#type={InputType::Text}
									start_text={Some("https://".to_string())}
									placeholder="URL"
								/>
							</div>

							<div class="flex-2">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input type="checkbox" />
									<p>"HTTP ONLY"</p>
								</label>
							</div>

							<div class="flex-3">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input type="checkbox" />
									<p>"PERMANENT REDIRECT"</p>
								</label>
							</div>
						}.into_view(),
						Ok(ManagedUrlTypeDiscriminant::ProxyUrl) => view! {
							<div class="flex-7">
								<Input
									variant={SecondaryColorVariant::Medium}
									r#type={InputType::Text}
									start_text={Some("https://".to_string())}
									placeholder="URL"
								/>
							</div>

							<div class="flex-2">
								<label class="flex items-center justify-center gap-sm w-full rounded-sm bg-secondary-medium row-card">
									<input type="checkbox" />
									<p>"HTTP ONLY"</p>
								</label>
							</div>
						}.into_view(),
						_ => view! {
							<div class="flex-9">
								<Input
									variant={SecondaryColorVariant::Medium}
									r#type={InputType::Text}
									start_text={Some("https://".to_string())}
									placeholder="URL"
								/>
							</div>
						}.into_view()
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
