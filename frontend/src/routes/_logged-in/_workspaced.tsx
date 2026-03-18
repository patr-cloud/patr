import { createFileRoute, Outlet, useNavigate } from "@tanstack/solid-router";
import { createEffect } from "solid-js";
import { useFetchWorkspaces, useFetchUserPermissions } from "~/hooks/fetch";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import Sidebar from "~/components/sidebar";
import TopBar from "~/components/top-bar";

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
					<Outlet />
				</div>
			</div>
		</main>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced")({
	component: WorkspacedLayout,
});
