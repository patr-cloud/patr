import { createRootRouteWithContext, Outlet } from "@tanstack/solid-router";
import type { RouterContext } from "~/router";
import NotFound from "./-not-found";

export const Route = createRootRouteWithContext<RouterContext>()({
	component: () => <Outlet />,
	notFoundComponent: NotFound,
});
