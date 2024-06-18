use crate::imports::*;

#[component]
fn ModalContainer(
	has_backdrop: bool,
	/// The Variant of the backdrop
	#[prop(into, optional)]
	variant: SecondaryColorVariant,
	/// The Content of the modal
	children: ChildrenFn,
) -> impl IntoView {
	view! {
		<>

			{if has_backdrop {
				view! { <Backdrop variant={variant}>{children()}</Backdrop> }.into_view()
			} else {
				view! { <>{children()}</> }.into_view()
			}}
		</>
	}
}

#[component]
pub fn Modal(
	/// The Content of the modal
	children: ChildrenFn,
	/// The Variant of the backdrop
	#[prop(optional)]
	color_variant: SecondaryColorVariant,
) -> impl IntoView {
	let children = store_value(children);
	view! {
		<Portal>
			<ModalContainer variant={color_variant} has_backdrop=true>
				{children.with_value(|children| children())}
			</ModalContainer>
		</Portal>
	}
}
