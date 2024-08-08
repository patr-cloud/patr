use crate::imports::*;

#[component]
pub fn OtpInput(
	/// Additional classes to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The length of the otp input
	#[prop(into, optional, default = 6)]
	_length: u32,
) -> impl IntoView {
	let class =
		class.with(|cname| format!("w-full flex items-center justify-center gap-xs {cname}"));

	view! { <div class={class}></div> }
}
