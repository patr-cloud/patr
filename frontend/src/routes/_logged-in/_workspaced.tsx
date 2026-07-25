import { createFileRoute, Outlet, useLocation } from "@tanstack/solid-router";
import { createEffect, ErrorBoundary, lazy, on, Show } from "solid-js";
import { useWorkspacesQuery, useUserPermissionsQuery } from "~/hooks/fetch";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { Sidebar } from "~/components";
import { useSidebar } from "~/components/sidebar/context";
import { IS_CLOUD } from "~/utils/env";

// The first-workspace screen is cloud-only and lazy + IS_CLOUD-gated so it (and
// the create-workspace form it pulls in) tree-shakes out of the self-hosted
// bundle, which has no create-workspace flow.
const CreateFirstWorkspace = IS_CLOUD ? lazy(() => import("~/components/create-first-workspace")) : null;

const WorkspacedLayout = () => {
	const workspacesQuery = useWorkspacesQuery();
	useUserPermissionsQuery();
	const location = useLocation();
	const [workspaceId, setWorkspaceId] = useLastWorkspaceId();
	const sidebar = useSidebar();

	createEffect(() => {
		if (!workspacesQuery.isPending) {
			const ws = workspacesQuery.data?.workspaces;
			if (ws && ws.length > 0) {
				if (!workspaceId()) {
					setWorkspaceId(ws[0].id);
				} else if (!ws.some((w) => w.id === workspaceId())) {
					// The cookie points at a workspace the user is no longer in
					// (removed by an owner, deleted, or never existed). Fall back to
					// the first workspace we DO have access to so the rest of the
					// tree doesn't 403 on every query.
					setWorkspaceId(ws[0].id);
				}
			}
		}
	});

	// Close the mobile sidebar whenever the route changes.
	createEffect(
		on(
			() => location().pathname,
			() => sidebar.setMobileOpen(false)
		)
	);

	const hasNoWorkspace = () => !workspacesQuery.isPending && (workspacesQuery.data?.workspaces?.length ?? 0) === 0;

	return (
		<div class="flex h-full">
			<Sidebar />
			<Show when={sidebar.isMobileOpen()}>
				<div
					class="fixed inset-0 bg-black/50 z-30 md:hidden"
					onClick={() => sidebar.setMobileOpen(false)}
					aria-hidden="true"
				/>
			</Show>
			<div class="flex-1 min-w-0 overflow-auto">
				<ErrorBoundary
					fallback={(err) => (
						<div class="flex items-center justify-center h-full text-white">
							<p>Something went wrong: {err.message}</p>
						</div>
					)}
				>
					<Show
						when={!hasNoWorkspace()}
						fallback={
							CreateFirstWorkspace ? (
								<CreateFirstWorkspace />
							) : (
								<div class="flex items-center justify-center h-full text-white p-8 text-center">
									<p>
										Self-hosted workspace is not initialised. Ask your administrator to seed the
										workspace before signing in.
									</p>
								</div>
							)
						}
					>
						<Outlet />
					</Show>
				</ErrorBoundary>
			</div>
		</div>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced")({
	component: WorkspacedLayout,
});
