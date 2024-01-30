use crate::imports::*;

#[component]
pub fn ContainerBody(
	/// The Children of the component
	children: Children,
	/// Additional Classnames to be given to the outer div
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"pos-rel fc-fs-ct full-width full-height ofy-auto container-body {}",
			class.get()
		)
	};

	view! {
		<div class=class>
			{children()}
			// <div class="fc-ct-ct txt-warning txt-lg gap-xxs txt-center p-xxl">
			//     "You DO NOT have permission to view this resource."
			//     <small class="txt-grey txt-center">
			//         "If you think this is a mistake, contact the admin of this workspace"
			//     </small>
			// </div>
		</div>
	}
}
