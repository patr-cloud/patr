import { createFileRoute, Outlet, useNavigate } from "@tanstack/solid-router";
import { createEffect, ErrorBoundary } from "solid-js";
import { useFetchWorkspaces, useFetchUserPermissions } from "~/hooks/fetch";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { Sidebar, TopBar } from "~/components";

const WorkspacedLayout = () => {
	const [workspaces] = useFetchWorkspaces();
	useFetchUserPermissions();
	const navigate = useNavigate();
	const [workspaceId, setWorkspaceId] = useLastWorkspaceId();

	createEffect(() => {
		if (workspaces.state === "ready") {
			const ws = workspaces()?.workspaces;
			if (!ws || ws.length === 0) {
				navigate({ to: "/onboard", replace: true });
			} else if (!workspaceId()) {
				setWorkspaceId(ws[0].id);
			}
		}
	});

	return (
		<main class="bg-secondary w-full min-h-screen h-screen flex">
			<Sidebar />
			<div class="flex-1 flex flex-col overflow-hidden">
				<TopBar />
				<div class="flex-1 overflow-auto">
					<ErrorBoundary
						fallback={(err) => (
							<div class="flex items-center justify-center h-full text-white">
								<p>Something went wrong: {err.message}</p>
							</div>
						)}
					>
						<Outlet />
					</ErrorBoundary>
				</div>
			</div>
		</main>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced")({
	component: WorkspacedLayout,
});
