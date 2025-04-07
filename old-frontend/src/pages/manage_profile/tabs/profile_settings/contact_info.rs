use crate::prelude::*;

#[component]
pub fn ContactInfo(
	/// Basic User with Id
	// #[prop(into)]
	user_email: Option<String>,
) -> impl IntoView {
	view! {
		<section class="text-white flex flex-col items-start justify-start w-full px-xl py-lg br-sm bg-secondary-light">
			<div class="flex items-center justify-start w-full pb-sm border-b border-border-color">
				<h2 class="tracking-[1px] text-md">"Contact Info"</h2>
			</div>

			<div class="w-full flex items-start justify-start gap-md pt-md">
				<div class="flex w-full px-md">
					<div class="flex-col-2 flex items-start justify-start">
						<label html_for="primaryEmail" class="mt-sm txt-sm">
							"Primary Email"
						</label>
					</div>

					<div class="flex-col-10 flex flex-col items-start justify-start">

						{match user_email {
							Some(email) => {
								view! {
									<InputDropdown
										variant={SecondaryColorVariant::Medium}
										placeholder={"Current Email".to_owned()}
										value={email}
										options={vec![]}
									/>
								}
									.into_view()
							}
							None => view! {}.into_view(),
						}}

					// <small class="txt-xxs txt-grey mt-xs">
					// "Add a new recovery email to change your primary email."
					// </small>
					</div>
				</div>

			// <div class="flex full-width px-md">
			// <div class="flex-col-2 fr-fs-fs">
			// <label html_for="primaryEmail" class="mt-sm txt-sm">
			// "Recovery Email(s)"
			// </label>
			// </div>

			// <div class="flex-col-10 fc-fs-fs gap-xs">
			// <EmailCard email="ac380012@gmail.com".to_owned() />
			// <form class="flex full-width">
			// <div class="flex-col-11 fc-fs-fs">
			// <Input
			// id="newEmail"
			// class="full-width"
			// r#type=InputType::Email
			// placeholder="Enter Email address"
			// variant=SecondaryColorVariant::Medium
			// />
			// </div>

			// <div class="flex-col-1 fr-ct-fs">
			// <Link
			// style_variant=LinkStyleVariant::Contained
			// should_submit=true
			// class="br-sm p-xs"
			// >
			// <Icon
			// icon=IconType::Plus
			// color=Color::Secondary
			// />
			// </Link>
			// </div>

			// </form>
			// </div>
			// </div>
			</div>
		</section>
	}
}
