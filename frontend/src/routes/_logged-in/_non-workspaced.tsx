import { createFileRoute, Outlet } from "@tanstack/solid-router";

const NonWorkspacedLayout = () => <Outlet />;

export const Route = createFileRoute("/_logged-in/_non-workspaced")({
	component: NonWorkspacedLayout,
});
