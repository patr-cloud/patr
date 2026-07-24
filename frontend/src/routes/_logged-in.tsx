import { createFileRoute, Outlet, redirect } from "@tanstack/solid-router";
import { createSignal } from "solid-js";
import { TopBar } from "~/components";
import { SidebarContext } from "~/components/sidebar/context";

const LoggedInLayout = () => {
	// The sidebar's open/close state lives here — above both the workspaced
	// layout that renders the sidebar and the topbar that toggles it — so the
	// topbar's mobile menu button can drive the sidebar in the workspaced zone.
	const [isMobileOpen, setMobileOpen] = createSignal(false);
	const sidebarCtx = {
		isMobileOpen,
		setMobileOpen,
		toggleMobile: () => setMobileOpen(!isMobileOpen()),
	};

	return (
		<SidebarContext.Provider value={sidebarCtx}>
			<div class="bg-secondary w-full h-screen flex flex-col">
				<TopBar />
				<main class="flex-1 min-h-0 overflow-auto">
					<Outlet />
				</main>
			</div>
		</SidebarContext.Provider>
	);
};

export const Route = createFileRoute("/_logged-in")({
	beforeLoad: ({ context }) => {
		if (!context.auth || context.auth.type === "LoggedOut") {
			throw redirect({ to: "/login" });
		}
	},
	component: LoggedInLayout,
});
