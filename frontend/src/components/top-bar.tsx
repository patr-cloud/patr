import { Link, useMatches, useRouter } from "@tanstack/solid-router";
import { Component, Show } from "solid-js";
import { FiMenu, FiArrowLeft } from "solid-icons/fi";
import UserDropdown from "./user-dropdown";
import { useSidebar } from "./sidebar/context";

const TopBar: Component = () => {
	const sidebar = useSidebar();
	const matches = useMatches();
	const router = useRouter();

	// Workspaced pages render the sidebar directly below the logo cell;
	// non-workspaced pages (profile/settings) don't. Several bits of chrome key
	// off this: the logo-cell border that continues the sidebar's vertical line,
	// the mobile sidebar toggle, and the back-to-home button that stands in for
	// the missing sidebar nav.
	const isWorkspaced = () => matches().some((m) => m.routeId.includes("_workspaced"));

	const goBack = () => {
		// Prefer real history so "back" returns wherever the user came from; fall
		// back to home when settings was opened via a fresh tab or a deep link.
		if (typeof window !== "undefined" && window.history.length > 1) {
			router.history.back();
		} else {
			router.navigate({ to: "/" });
		}
	};

	return (
		<header class="h-14 md:h-16 shrink-0 bg-secondary border-b border-white/5 flex items-stretch">
			{/* Logo cell — same width and right border as the sidebar so, on a
			    workspaced page, the border reads as one continuous vertical line
			    from the top of the topbar down through the sidebar. */}
			<div
				class={`flex items-center shrink-0 px-3 md:px-0 md:w-14 md:justify-center lg:w-64 lg:justify-start lg:px-6 ${
					isWorkspaced() ? "md:border-r md:border-white/5" : ""
				}`}
			>
				<Link to="/" aria-label="Go to home">
					<img src="/images/patr-lowercase.png" alt="Patr Cloud" class="h-8 w-auto" />
				</Link>
			</div>

			{/* Main region — leading actions on the left, user menu on the right. */}
			<div class="flex-1 flex items-center justify-between px-3 md:px-6 gap-2 min-w-0">
				<div class="flex items-center gap-2">
					<Show when={isWorkspaced()}>
						<button
							type="button"
							class="md:hidden text-white p-2 rounded-xs hover:bg-white/5"
							aria-label="Open navigation menu"
							onClick={() => sidebar.toggleMobile()}
						>
							<FiMenu size={20} />
						</button>
					</Show>
					<Show when={!isWorkspaced()}>
						<button
							type="button"
							class="flex items-center gap-2 text-gray-300 hover:text-white px-2 py-2 rounded-xs hover:bg-white/5 transition-colors"
							aria-label="Back to home"
							onClick={goBack}
						>
							<FiArrowLeft size={18} />
							<span class="hidden sm:inline text-sm">Back</span>
						</button>
					</Show>
				</div>
				<UserDropdown />
			</div>
		</header>
	);
};

export default TopBar;
