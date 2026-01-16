import { A, useLocation } from "@solidjs/router";
import { FiHome, FiBox, FiDatabase, FiCpu, FiGlobe, FiSettings, FiChevronDown, FiChevronRight } from "solid-icons/fi";
import { Component, createSignal, For, Show } from "solid-js";
import WorkspaceSwitcher from "./workspace-switcher";

interface SidebarItemProps {
	label: string;
	href?: string;
	icon: Component;
	children?: SidebarItemProps[];
}

const SidebarItem = (props: SidebarItemProps) => {
	const location = useLocation();
	const [isOpen, setIsOpen] = createSignal(false);

	const isActive = () => {
		if (props.href) {
			return location.pathname === props.href;
		}
		return props.children?.some((child) => child.href === location.pathname);
	};

	const handleClick = (e: MouseEvent) => {
		if (props.children) {
			e.preventDefault();
			setIsOpen(!isOpen());
		}
	};

	return (
		<div class="w-full">
			<A
				href={props.href || "#"}
				class={`flex items-center justify-between px-4 py-3 text-sm font-medium transition-colors duration-200 ${
					isActive()
						? "text-white bg-primary/10 border-r-2 border-primary"
						: "text-gray-400 hover:text-white hover:bg-white/5"
				}`}
				onClick={handleClick}
			>
				<div class="flex items-center gap-3">
					<div class="w-[18px] h-[18px] flex items-center justify-center">
						<props.icon />
					</div>
					<span>{props.label}</span>
				</div>
				<Show when={props.children}>
					<div class="text-gray-500">{isOpen() ? <FiChevronDown /> : <FiChevronRight />}</div>
				</Show>
			</A>
			<Show when={props.children && isOpen()}>
				<div class="bg-black/20">
					<For each={props.children}>
						{(child) => (
							<A
								href={child.href || "#"}
								class={`flex items-center gap-3 pl-11 pr-4 py-2 text-sm transition-colors duration-200 ${
									location.pathname === child.href
										? "text-white bg-white/5"
										: "text-gray-400 hover:text-white hover:bg-white/5"
								}`}
							>
								<span>{child.label}</span>
							</A>
						)}
					</For>
				</div>
			</Show>
		</div>
	);
};

const Sidebar: Component = () => {
	const items: SidebarItemProps[] = [
		{
			label: "Home",
			href: "/",
			icon: FiHome,
		},
		{
			label: "Deployments",
			href: "/deployments",
			icon: FiBox,
		},
		{
			label: "Databases",
			href: "/databases",
			icon: FiDatabase,
		},
		{
			label: "Runners",
			href: "/runners",
			icon: FiCpu,
		},
		{
			label: "Domains",
			icon: FiGlobe,
			href: "/domains",
		},
		{
			label: "Workspace Settings",
			href: "/workspaces",
			icon: FiSettings,
		},
	];

	return (
		<aside class="w-64 h-screen bg-secondary border-r border-white/5 flex flex-col">
			<div class="p-6 flex items-center gap-3">
				<img src="/images/patr-lowercase.png" alt="Patr Cloud" class="h-8 w-auto" />
			</div>

			<nav class="flex-1 overflow-y-auto py-4">
				<For each={items}>{(item) => <SidebarItem {...item} />}</For>
			</nav>

			<div class="p-4 border-t border-white/5">
				<WorkspaceSwitcher />
			</div>
		</aside>
	);
};

export default Sidebar;
