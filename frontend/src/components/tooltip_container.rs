use crate::prelude::*;

#[component]
pub fn TooltipContainer(
	/// The content of the tooltip
	#[prop(into, optional)]
	content: String,
	/// The label to display in the container
	#[prop(optional)]
	label: Option<Children>,
	/// Whether to disable focus on the container
	/// (useful for when the container is not visible)
	#[prop(optional)]
	disable_focus: bool,
	/// The color variant of the icon in the tooltip
	#[prop(into, optional)]
	icon_color: Color,
	/// The color variant of the tooltip
	#[prop(into, optional)]
	variant: SecondaryColorVariant,
	/// Additional class names to add to the container
	#[prop(into, optional)]
	class: String,
	/// The children of the container
	#[prop(into)]
	children: ChildrenFn,
) -> impl IntoView {
	let container_ref = create_node_ref::<html::Span>();

	view! {
		<span
			ref={container_ref}
			tab_index=if disable_focus { -1 } else { 0 }
			class=format!(
				"fr-ct-ct pos-rel br-sm mx-xxs tooltip-container {} {class}",
				if disable_focus {
					"enable-focus"
				} else {
					""
				}
			)
		>
			{if let Some(label) = label {
				label()
			} else {
				view! {
					<Icon
						icon=IconType::Info
						size={Size::ExtraSmall}
						color={icon_color}
						class="br-round cursor-pointer"
					/>
				}.into()
			}}
			<Tooltip
				content={content}
				parent_ref={container_ref}
				children={children}
				variant={variant}
			/>
		</span>
	}
}
