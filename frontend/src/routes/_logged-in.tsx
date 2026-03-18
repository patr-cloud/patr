import { createFileRoute, Outlet, redirect } from "@tanstack/solid-router";

const LoggedInLayout = () => <Outlet />;

export const Route = createFileRoute("/_logged-in")({
	beforeLoad: ({ context }) => {
		if (!context.auth || context.auth.type === "LoggedOut") {
			throw redirect({ to: "/login" });
		}
	},
	component: LoggedInLayout,
});
