use crate::imports::*;

#[component]
pub fn ToolTipContainer(
	/// The content of the tooltip
	#[prop(into, optional)]
	content: String,
	/// The label of the display in the container
	#[prop(optional)]
	label: Option<View>,
	/// The Color Variant of the icon in the tooltip
	#[prop(into, optional)]
	icon_color: Color,
	/// The color variant of the tooltip
	#[prop(into, optional)]
	color_variant: SecondaryColorVariant,
	/// Additional class names to add to the container
	#[prop(into, optional)]
	class: String,
	/// The Children of the container
	children: ChildrenFn,
	/// The Width of the tooltip, in rem.
	#[prop(optional, default = 16.)]
	tooltip_width: f64,
) -> impl IntoView {
	let container_ref = NodeRef::<html::Span>::new();
	view! {
		<span
			r#ref={container_ref}
			class={format!("fr-ct-ct pos-rel br-sm mx-xxs tooltip-container {class}")}
		>

			{if let Some(label) = label {
				label.into_view()
			} else {
				view! {
					<Icon
						icon={IconType::Info}
						size={Size::ExtraSmall}
						color={icon_color}
						class="br-round cursor-pointer"
					/>
				}
					.into_view()
			}}

			<Tooltip
				content={content}
				parent_ref={container_ref}
				variant={color_variant}
				children={children}
				width={tooltip_width}
			/>
		</span>
	}
}
