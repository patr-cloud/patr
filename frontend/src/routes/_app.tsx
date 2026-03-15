import { createFileRoute, Outlet, redirect } from "@tanstack/solid-router";
import { createEffect, createResource } from "solid-js";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useToast } from "~/components";
import { httpRequest } from "~/utils/http-request";
import { ListUserWorkspacesResponse } from "~/bindings";
import Sidebar from "~/components/sidebar";
import TopBar from "~/components/top-bar";
import { useAuthState } from "~/hooks";

const AppLayout = () => {
	const [authState] = useAuthState();
	const [workspaceId, setWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const [workspaceResource] = createResource(authState, async (auth) => {
		if (auth === null || auth.type !== "LoggedIn") {
			return { workspaces: [] };
		}
		const response = await httpRequest<ListUserWorkspacesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch workspaces:", response.data.error);
			toast("Failed to fetch workspaces", "error");
			return { workspaces: [] };
		}

		return response.data;
	});

	createEffect(() => {
		if (!workspaceId()) {
			const workspaces = workspaceResource();
			if (workspaces && workspaces.workspaces.length > 0) {
				setWorkspaceId(workspaces.workspaces[0].id);
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

export const Route = createFileRoute("/_app")({
	beforeLoad: ({ context }) => {
		if (!context.auth || context.auth.type === "LoggedOut") {
			throw redirect({ to: "/login" });
		}
	},
	component: AppLayout,
});
