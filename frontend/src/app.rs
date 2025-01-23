use leptos_router::components::{Outlet, ParentRoute, ProtectedRoute, Route, Router, Routes};

use crate::{pages::*, prelude::*, utils::AuthState};

/// The View for the App Component, it encapsulates the whole application, and
/// adds the sidebar or header if necessary
#[component]
pub fn AppOutletView() -> impl IntoView {
	let (state, _) = AuthState::load();
	let app_type = expect_context::<AppType>();

	move || match state.get() {
		AuthState::LoggedOut => view! {
			<PageContainer class="bg-image">
				<Outlet />
			</PageContainer>
		}
		.into_any(),
		AuthState::LoggedIn { .. } => view! {
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
		.into_any(),
	}
}

#[component(transparent)]
fn RunnerWorkspaceRoutes() -> impl IntoView {
	view! {
		<ParentRoute path="" view={Outlet}>
			<ParentRoute path={LoggedInRoute::Runners} view={RunnerPage}>
				<Route path="create" view={CreateRunner} />
				<Route path=":runner_id" view={ManageRunner} />
				<Route path={AppRoutes::Empty} view={RunnerDashboard} />
			</ParentRoute>
			<ParentRoute path={AppRoutes::Empty} view={|| view! { <Outlet /> }}>
				<ParentRoute path={LoggedInRoute::Workspace} view={WorkspacePage}>
					<ParentRoute path={AppRoutes::Empty} view={ManageWorkspace}>
						<Route path="" view={ManageWorkspaceSettingsTab} />
					</ParentRoute>
					<Route path="/create" view={CreateWorkspace} />
				</ParentRoute>
			</ParentRoute>
		</ParentRoute>
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
	provide_toaster();

	view! {
		<Toaster />

		<Router>
			<Routes>
				// Logged in routes
				<ProtectedRoute
					path={AppRoutes::Empty}
					view={AppOutletView}
					redirect_path={|| AppRoutes::LoggedOutRoute(LoggedOutRoute::Login)}
					condition={move || state.get().is_logged_in()}
				>
					<ProfileRoutes />
					<InfrastructureRoutes />
					<ParentRoute path={LoggedInRoute::ManagedUrl} view={ManagedUrlPage}>
						<Route path="create" view={|| view! { <div>"create"</div> }} />
						<Route path={AppRoutes::Empty} view={UrlDashboard} />
					</ParentRoute>
					<Route path={LoggedInRoute::Domain} view={DomainsDashboard} />
					{app_type.is_managed().then(RunnerWorkspaceRoutes)}
					<Route
						path={AppRoutes::Empty}
						view={HomePage}
					/>
				</ProtectedRoute>
				<ProtectedRoute
					path={"".to_string()}
					redirect_path={|| AppRoutes::LoggedInRoute(LoggedInRoute::Home).to_string()}
					view={AppOutletView}
					condition={move || state.get().is_logged_out()}
				>
					<AppRoute<LoginRoute, _, _> view={|query, _| LoginForm(LoginFormProps { query })} />
					<AppRoute<SignUpRoute, _, _> view={|query, _| SignUpForm(SignUpFormProps { query })} />
					{app_type
						.is_managed()
						.then(|| {
							view! { <Route path="/confirm" view={ConfirmSignUpPage} /> }
						})}
				</ProtectedRoute>
				<Route
					path="/*any"
					view={|| {
						view! {
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
					}}
				/>
			</Routes>
		</Router>
	}
}
