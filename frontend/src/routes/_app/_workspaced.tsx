import { createFileRoute, Outlet, useNavigate } from "@tanstack/solid-router";
import { createEffect, ParentProps } from "solid-js";
import { useFetchWorkspaces, useFetchUserPermissions } from "~/hooks/fetch";

const WorkspacedLayout = () => {
	const [workspaces] = useFetchWorkspaces();
	useFetchUserPermissions();
	const navigate = useNavigate();

	createEffect(() => {
		if (workspaces.state === "ready") {
			console.log("workspaces:", workspaces());
			const workspaceLength = workspaces()?.workspaces?.length || 0;
			if (workspaceLength === 0) {
				navigate({ to: "/onboard", replace: true });
			}
		}
	});

	return <Outlet />;
};

export const Route = createFileRoute("/_app/_workspaced")({
	component: WorkspacedLayout,
});
