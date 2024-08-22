use crate::imports::*;

/// The Toast component. The toast component is used to display a notification
/// to the user. For Example, to show the user a notification for success
/// or failure of an action like reseouce creation or deletion.
#[component]
pub fn Toast(
	/// Additional classes to add to the toast
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Whether the toast is visible or not, when true, the toast will be shown
	show: RwSignal<bool>,
	/// The Title of the toast
	#[prop(into, optional)]
	title: MaybeSignal<String>,
	/// The Children of the component
	children: ChildrenFn,
) -> impl IntoView {
	let class = move || {
		with!(|class, show| {
			format!("
                {} {} popup br-sm fixed flex flex-col items-start justify-start text-white outline-primary-focus", 
				class, 
				if *show {"show"} else {""}
			)
		})
	};
	view! {
		<div class={class}>
			<p class="text-primary mr-xs">
				{title}
			</p>
			<div class="w-full h-full overflow-y-auto fc-fs-fs px-lg pb-md pt-xxs">
				{children}
			</div>
		</div>
	}
}
