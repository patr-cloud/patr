import { A } from "@solidjs/router";
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
		<A
			href={props.href}
			class="group relative flex flex-col gap-4 rounded-sm bg-secondary-light p-6 border border-white/5 transition-all duration-300 hover:border-primary/30 hover:bg-secondary-medium/50"
		>
			<div class={`flex h-12 w-12 items-center justify-center rounded-xs ${props.color}`}>
				<props.icon />
			</div>
			<div class="flex flex-col gap-1">
				<h3 class="text-base font-medium text-white group-hover:text-primary transition-colors">{props.title}</h3>
				<p class="text-sm text-gray-400 leading-relaxed">{props.description}</p>
			</div>
			<div class="flex items-center gap-1 text-sm text-primary opacity-0 group-hover:opacity-100 transition-opacity duration-300">
				<span>Get started</span>
				<FiArrowRight class="transition-transform group-hover:translate-x-1" />
			</div>
		</A>
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
		<A
			href={props.href}
			class="group flex items-center gap-4 rounded-xs bg-secondary-light/50 p-4 border border-white/5 transition-all duration-200 hover:bg-secondary-medium/30 hover:border-white/10"
		>
			<div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xs bg-white/5 text-gray-400 group-hover:text-primary transition-colors">
				<props.icon />
			</div>
			<div class="flex flex-col gap-0.5 min-w-0">
				<span class="text-sm font-medium text-white">{props.title}</span>
				<span class="text-xs text-gray-500">{props.description}</span>
			</div>
			<FiArrowRight class="ml-auto shrink-0 text-gray-600 group-hover:text-primary transition-colors" />
		</A>
	);
};

const HomePage = () => {
	const quickActions: QuickActionProps[] = [
		{
			title: "Set Up a Runner",
			description: "Connect your own infrastructure to Patr. Runners execute deployments on your machines or clusters.",
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
			href: "/workspace-settings",
			icon: FiZap,
		},
	];

	return (
		<PageContainer>
			<PageContainerHead title="Workspace" subText="Here's what you can do to get started" subTitle="Home" />

			<PageContainerBody class="overflow-y-auto">
				<div class="flex flex-col gap-8 max-w-5xl">
					{/* Welcome Section */}
					<div class="flex flex-col gap-2">
						<h2 class="text-2xl font-semibold text-white">Patr</h2>
						<p class="text-gray-400 text-sm max-w-2xl leading-relaxed">
							Your DevOps platform for deploying and managing containerized applications. Get started by creating a
							deployment, setting up a runner, or connecting a domain.
						</p>
					</div>

					{/* Quick Actions */}
					<div class="flex flex-col gap-4">
						<div class="flex items-center gap-2">
							<FiPlus class="text-primary" />
							<h3 class="text-sm font-semibold text-gray-300 uppercase tracking-wide">Quick Actions</h3>
						</div>
						<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
							<For each={quickActions}>{(action) => <QuickActionCard {...action} />}</For>
						</div>
					</div>

					{/* Resources */}
					<div class="flex flex-col gap-4">
						<div class="flex items-center gap-2">
							<FiBookOpen class="text-primary" />
							<h3 class="text-sm font-semibold text-gray-300 uppercase tracking-wide">Resources</h3>
						</div>
						<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
							<For each={resources}>{(resource) => <ResourceLink {...resource} />}</For>
						</div>
					</div>
				</div>
			</PageContainerBody>
		</PageContainer>
	);
};

export default HomePage;
