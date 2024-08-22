use crate::imports::*;

#[component]
pub fn ErrorPage(
	/// The Title of the error page
	#[prop(into)]
	title: String,
	/// The Content of the error page
	#[prop(into, optional, default = None.into())]
	content: Option<View>,
) -> impl IntoView {
	view! {
		<div class="w-full h-full flex flex-col justify-start items-start bg-empty">
			<div class="w-full h-full flex flex-col items-center justify-start pt-[10rem] gap-md">
				<h2 class="text-primary text-2xl font-bold">{title}</h2>
				<div>{content}</div>
			</div>
		</div>
	}
}
