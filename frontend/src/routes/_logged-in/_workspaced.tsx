import { createFileRoute, Outlet, useLocation, useNavigate } from "@tanstack/solid-router";
import { createEffect, createSignal, ErrorBoundary, on, Show } from "solid-js";
import { useWorkspacesQuery, useUserPermissionsQuery } from "~/hooks/fetch";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { Sidebar, TopBar } from "~/components";
import { SidebarContext } from "~/components/sidebar/context";
import { IS_CLOUD } from "~/utils/env";

const WorkspacedLayout = () => {
	const workspacesQuery = useWorkspacesQuery();
	useUserPermissionsQuery();
	const navigate = useNavigate();
	const location = useLocation();
	const [workspaceId, setWorkspaceId] = useLastWorkspaceId();
	const [isMobileOpen, setMobileOpen] = createSignal(false);

	createEffect(() => {
		// Wait until the query has actually settled — `isPending` only covers
		// the initial fetch, so without `isFetching` an invalidation triggered
		// by /onboard briefly exposes stale empty data and bounces the user
		// back, creating a microtask-rate redirect loop.
		if (!workspacesQuery.isPending && !workspacesQuery.isFetching) {
			const ws = workspacesQuery.data?.workspaces;
			if (!ws || ws.length === 0) {
				// Stored workspace no longer exists / user no longer has access
				// — clear it so dependent queries (permissions etc.) don't fire
				// against a stale id and 401.
				if (workspaceId()) {
					setWorkspaceId(null);
				}
				// On self-hosted there is no onboarding flow — the singleton
				// workspace is seeded out-of-band. Render an inline error
				// instead of redirecting into a 404 loop.
				if (IS_CLOUD) {
					navigate({ to: "/onboard", replace: true });
				}
			} else {
				const current = workspaceId();
				if (!current || !ws.some((w) => w.id === current)) {
					setWorkspaceId(ws[0].id);
				}
			}
		}
	});

	createEffect(
		on(
			() => location().pathname,
			() => setMobileOpen(false)
		)
	);

	const sidebarCtx = {
		isMobileOpen,
		setMobileOpen,
		toggleMobile: () => setMobileOpen(!isMobileOpen()),
	};

	const noWorkspaceOnSelfHosted = () =>
		!IS_CLOUD &&
		!workspacesQuery.isPending &&
		(workspacesQuery.data?.workspaces?.length ?? 0) === 0;

	return (
		<SidebarContext.Provider value={sidebarCtx}>
			<main class="bg-secondary w-full min-h-screen h-screen flex">
				<Sidebar />
				<Show when={isMobileOpen()}>
					<div
						class="fixed inset-0 bg-black/50 z-30 md:hidden"
						onClick={() => setMobileOpen(false)}
						aria-hidden="true"
					/>
				</Show>
				<div class="flex-1 flex flex-col overflow-hidden min-w-0">
					<TopBar />
					<div class="flex-1 overflow-auto">
						<ErrorBoundary
							fallback={(err) => (
								<div class="flex items-center justify-center h-full text-white">
									<p>Something went wrong: {err.message}</p>
								</div>
							)}
						>
							<Show
								when={!noWorkspaceOnSelfHosted()}
								fallback={
									<div class="flex items-center justify-center h-full text-white p-8 text-center">
										<p>
											Self-hosted workspace is not initialised. Ask your administrator to seed the
											workspace before signing in.
										</p>
									</div>
								}
							>
								<Outlet />
							</Show>
						</ErrorBoundary>
					</div>
				</div>
			</main>
		</SidebarContext.Provider>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced")({
	component: WorkspacedLayout,
});
