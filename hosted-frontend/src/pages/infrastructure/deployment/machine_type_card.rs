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
                "px-xl py-lg bg-secondary-medium cursor-pointer rounded-sm flex flex-col items-start justify-start machine-type-card {} {}",
				cname,
				if is_selected.get() { "bd-primary" } else { "bd-none" }
			)
		})
	};
	create_effect(move |_| {
		logging::log!("{}", is_selected.get());
	});

	view! {
		<div class={outer_div_class} on:click={
			let id = machine_type.get().id;
			is_selected.update(|v| *v = !*v);
			move |_| {
				on_select.call(id);
			}
		}>
			<div class="flex justify-start items-baseline">
				<span class="text-md">
					{format!("{} MB", machine_type.clone().get().memory_count)}
				</span>
				<span class="text-disabled ml-xxs text-xxs">"RAM"</span>
			</div>
			<div class="flex justify-start items-baseline">
				<span class="text-lg">{machine_type.get().cpu_count}</span>
				<span class="text-disabled ml-xxs text-xxs">"vCPU"</span>
			</div>

		</div>
	}
}
