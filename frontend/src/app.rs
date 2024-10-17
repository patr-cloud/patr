use leptos_query_devtools::LeptosQueryDevtools;
use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};

use crate::{pages::*, prelude::*, utils::AuthState};

/// The View for the App Component, it encapsulates the whole application, and
/// adds the sidebar or headedr if necessary
#[component]
pub fn AppOutletView() -> impl IntoView {
	let (state, _) = AuthState::load();
	let app_type = expect_context::<AppType>();

	view! {
		{move || match state.get() {
			AuthState::LoggedOut => {
				view! {
					<PageContainer class="bg-image">
						<Outlet />
					</PageContainer>
				}
					.into_view()
			}
			AuthState::LoggedIn { .. } => {
				view! {
					<div class="fr-fs-fs full-width full-height bg-secondary">
						<Sidebar sidebar_items={get_sidebar_items(
							app_type,
						)}>
							{app_type
								.is_managed()
								.then(|| {
									view! {
										<Transition>
											<WorkspaceSidebarComponent />
										</Transition>
									}
								})}
						</Sidebar>

						<main class="fc-fs-ct full-width px-lg">
							<Outlet />
						</main>
					</div>
				}
					.into_view()
			}
		}}
	}
}

#[component(transparent)]
fn RunnerWorkspaceRoutes() -> impl IntoView {
	view! {
		<Route path="" view={|| view! {<Outlet />}}>
			<Route path={LoggedInRoute::Runners} view={RunnerPage}>
				<Route path={"create"} view={CreateRunner}/>
				<Route path={":runner_id"} view={ManageRunner}/>
				<Route path={AppRoutes::Empty} view={RunnerDashboard}/>
			</Route>
			<Route path={AppRoutes::Empty} view={|| view! { <Outlet /> }}>
				<Route path={LoggedInRoute::Workspace} view={WorkspacePage}>
					<Route path={AppRoutes::Empty} view={ManageWorkspace}>
						<Route path="" view={ManageWorkspaceSettingsTab} />
					</Route>
					<Route path="/create" view={CreateWorkspace} />
				</Route>
			</Route>
		</Route>
	}
}

/// The main application component. This is the root component of the
/// application. It contains the main router and all the routes.
#[component]
pub fn App() -> impl IntoView {
	let (state, _) = AuthState::load();
	let app_type = AppType::Managed;

	// TODO: When redirecting to login, the URL should include the path that the
	// user was trying to access. This way, after login, the user is redirected
	// to the page they were trying to access.
	provide_context(app_type);

	view! {
		<LeptosQueryDevtools />
		<Router>
			<Routes>
				// Logged in routes
				<ProtectedRoute
					path={AppRoutes::Empty}
					view={AppOutletView}
					redirect_path={AppRoutes::LoggedOutRoute(LoggedOutRoute::Login)}
					condition={move || state.get().is_logged_in()}
				>
					<ProfileRoutes />
					<InfrastructureRoutes />
					<Route path={LoggedInRoute::ManagedUrl} view={ManagedUrlPage}>
						<Route path="create" view={|| view! { <div>"create"</div> }} />
						<Route path={AppRoutes::Empty} view={UrlDashboard} />
					</Route>
					<Route path={LoggedInRoute::Domain} view={DomainsDashboard} />
					{app_type
						.is_managed()
						.then(|| {
							view! {
								<RunnerWorkspaceRoutes />
							}
						})
					}
					<Route
						path={AppRoutes::Empty}
						view={|| view! {
							<div class="pt-[25vh] w-full flex justify-center items-center">
								<h1 class="text-primary text-xl">"Welcome To Patr!"</h1>
							</div>
						}}
					/>
				</ProtectedRoute>
				<ProtectedRoute
					path={AppRoutes::Empty}
					redirect_path={AppRoutes::LoggedInRoute(LoggedInRoute::Home)}
					view={AppOutletView}
					condition={move || state.get().is_logged_out()}
				>
					<Route path="/login" view={LoginForm} />
					<Route path="/sign-up" view={SignUpForm} />
					{app_type
						.is_managed()
						.then(|| {
							view! {
								<Route path="/confirm" view={ConfirmSignUpPage} />
							}
						})
					}
				</ProtectedRoute>
				<Route path="/*any" view={
					|| view! {
						<ErrorPage
							title="Page Not Found"
							content={
								view! {
									<Link
										r#type={Variant::Link}
										style_variant={LinkStyleVariant::Contained}
										to="/"
									>
										"Go to Home"
									</Link>
								}
							}
						/>
					}
				} />
			</Routes>
		</Router>
	}
}
