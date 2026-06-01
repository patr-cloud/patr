import { Component } from "solid-js";
import { FiMenu } from "solid-icons/fi";
import UserDropdown from "./user-dropdown";
import { useSidebar } from "./sidebar/context";
import WorkspaceSwitcher from "./sidebar/workspace-switcher";

const TopBar: Component = () => {
	const sidebar = useSidebar();

	return (
		<header class="h-14 md:h-16 bg-secondary border-b border-white/5 flex items-center justify-between lg:justify-end px-3 md:px-6 gap-2">
			<div class="flex items-center gap-2 lg:hidden">
				<button
					type="button"
					class="md:hidden text-white p-2 rounded-xs hover:bg-white/5"
					aria-label="Open navigation menu"
					onClick={() => sidebar.toggleMobile()}
				>
					<FiMenu size={20} />
				</button>
				<WorkspaceSwitcher placement="bottom" compact />
			</div>
			<UserDropdown />
		</header>
	);
};

export default TopBar;
