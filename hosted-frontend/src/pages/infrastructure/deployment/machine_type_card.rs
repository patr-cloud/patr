use models::api::workspace::deployment::DeploymentMachineType;

use crate::prelude::*;

#[component]
pub fn MachineTypeCard(
	/// Additional classes to apply to the outer div if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Machine Type Info
	#[prop(into)]
	machine_type: MaybeSignal<WithId<DeploymentMachineType>>,
	/// On Selecting an Input
	#[prop(into, optional, default = Callback::new(|_| {}))]
	on_select: Callback<Uuid>,
) -> impl IntoView {
	let is_selected = create_rw_signal(false);

	let outer_div_class = move || {
		class.with(|cname| {
			format!(
                "px-xl py-lg bg-secondary-light cursor-pointer br-sm fc-fs-fs machine-type-card {} {}",
				cname,
				if is_selected.get() { "bd-none" } else { "bd-primary" }
			)
		})
	};

	view! {
		<label class="fr-fs-fs gap-md">
			<input
				class="mt-xs"
				type="radio"
				name="machine_type"
				value={machine_type.get().id.to_string()}
			/>

			<div class={outer_div_class} on:click={
				let id = machine_type.get().id.clone();
				is_selected.update(|v| *v = !*v);
				move |_| {
					on_select.call(id);
				}
			}>
				<div class="fr-fs-bl">
					<span class="txt-md">
						{format!("{} MB", machine_type.clone().get().memory_count)}
					</span>
					<span class="txt-disabled ml-xxs txt-xxs">"RAM"</span>
				</div>
				<div class="fr-fs-bl">
					<span class="txt-lg">{machine_type.get().cpu_count}</span>
					<span class="txt-disabled ml-xxs txt-xxs">"vCPU"</span>
				</div>

			</div>
		</label>
	}
}
