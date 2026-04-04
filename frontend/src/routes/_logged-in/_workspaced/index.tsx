import { createFileRoute, Link } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { Component, For } from "solid-js";
import { FiBox, FiCpu, FiGlobe, FiArrowRight, FiPlus, FiBookOpen, FiTerminal, FiLayers, FiZap } from "solid-icons/fi";
import { PageContainer, PageContainerBody, PageContainerHead } from "~/components";

interface QuickActionProps {
	title: string;
	description: string;
	href: string;
	icon: Component;
	color: string;
}

const QuickActionCard = (props: QuickActionProps) => {
	return (
		<Link
			to={props.href}
			class="group relative flex flex-col gap-4 rounded-sm bg-secondary-light p-6 border border-white/5 transition-all duration-300 hover:border-primary/30 hover:bg-secondary-medium/50"
		>
			<div class={`flex h-12 w-12 items-center justify-center rounded-xs ${props.color}`}>
				<props.icon />
			</div>
			<div class="flex flex-col gap-1">
				<h3 class="text-base font-medium text-white group-hover:text-primary transition-colors">
					{props.title}
				</h3>
				<p class="text-sm text-grey leading-relaxed">{props.description}</p>
			</div>
			<div class="flex items-center gap-1 text-sm text-primary opacity-0 group-hover:opacity-100 transition-opacity duration-300">
				<span>Get started</span>
				<FiArrowRight class="transition-transform group-hover:translate-x-1" />
			</div>
		</Link>
	);
};

interface ResourceLinkProps {
	title: string;
	description: string;
	href: string;
	icon: Component;
}

const ResourceLink = (props: ResourceLinkProps) => {
	return (
		<Link
			to={props.href}
			class="group flex items-center gap-4 rounded-xs bg-secondary-light/50 p-4 border border-white/5 transition-all duration-200 hover:bg-secondary-medium/30 hover:border-white/10"
		>
			<div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xs bg-white/5 text-grey group-hover:text-primary transition-colors">
				<props.icon />
			</div>
			<div class="flex flex-col gap-0.5 min-w-0">
				<span class="text-sm font-medium text-white">{props.title}</span>
				<span class="text-xs text-grey">{props.description}</span>
			</div>
			<FiArrowRight class="ml-auto shrink-0 text-grey group-hover:text-primary transition-colors" />
		</Link>
	);
};

const HomePage = () => {
	const quickActions: QuickActionProps[] = [
		{
			title: "Set Up a Runner",
			description:
				"Connect your own infrastructure to Patr. Runners execute deployments on your machines or clusters.",
			href: "/runners/new",
			icon: FiCpu,
			color: "bg-info/15 text-info",
		},
		{
			title: "Create a Deployment",
			description:
				"Deploy your containerized application in minutes. Configure ports, environment variables, and scaling options.",
			href: "/deployments/new",
			icon: FiBox,
			color: "bg-primary/15 text-primary",
		},
		{
			title: "Add a Domain",
			description: "Register a custom domain and configure DNS to route traffic to your deployments.",
			href: "/domains/new",
			icon: FiGlobe,
			color: "bg-success/15 text-success",
		},
	];

	const resources: ResourceLinkProps[] = [
		{
			title: "View Deployments",
			description: "Manage and monitor all your running services",
			href: "/deployments",
			icon: FiLayers,
		},
		{
			title: "Manage Runners",
			description: "View connected runners and their status",
			href: "/runners",
			icon: FiTerminal,
		},
		{
			title: "Domain Configuration",
			description: "Configure DNS and routing for your domains",
			href: "/domains",
			icon: FiGlobe,
		},
		{
			title: "Workspace Settings",
			description: "Manage team members, roles, and permissions",
			href: "/workspace",
			icon: FiZap,
		},
	];

	return (
		<>
			<Title>Home | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Home",
						},
					]}
					subText="Deploy and manage containerized applications across your infrastructure."
				/>

				<PageContainerBody class="overflow-y-auto">
					<div class="flex flex-col gap-8">
						{/* Quick Actions */}
						<div class="flex flex-col gap-4">
							<div class="flex items-center gap-2">
								<FiPlus class="text-primary" aria-hidden="true" />
								<h2 class="text-sm font-semibold text-grey uppercase tracking-wide">Quick Actions</h2>
							</div>
							<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
								<For each={quickActions}>{(action) => <QuickActionCard {...action} />}</For>
							</div>
						</div>

						{/* Resources */}
						<div class="flex flex-col gap-4">
							<div class="flex items-center gap-2">
								<FiBookOpen class="text-primary" aria-hidden="true" />
								<h2 class="text-sm font-semibold text-grey uppercase tracking-wide">Resources</h2>
							</div>
							<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
								<For each={resources}>{(resource) => <ResourceLink {...resource} />}</For>
							</div>
						</div>
					</div>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/")({
	component: HomePage,
});
