import { createFileRoute, Outlet, redirect } from "@tanstack/solid-router";
import { BgOnboard } from "~/components";

const AuthLayout = () => {
	return (
		<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
			<BgOnboard />
			<Outlet />
		</main>
	);
};

export const Route = createFileRoute("/_auth")({
	beforeLoad: ({ context }) => {
		if (context.auth?.type === "LoggedIn") {
			throw redirect({ to: "/" });
		}
	},
	component: AuthLayout,
});
