import { Link as RouterLink, useLocation } from "@tanstack/solid-router";
import { JSX } from "solid-js";
import { ButtonVariant, Link, PageContainerHead } from "~/components";

interface WorkspaceHeaderProps {
	workspaceName?: string;
	activeTab: "general" | "members" | "roles";
	/** Extra tab-specific header actions, rendered alongside the built-in ones. */
	actions?: () => JSX.Element;
}

const WorkspaceHeader = (props: WorkspaceHeaderProps) => {
	const location = useLocation();

	return (
		<PageContainerHead
			breadcrumbs={[
				{
					label: "Workspace Settings",
					url: props.activeTab !== "general" ? "/workspace" : undefined,
				},
				...(props.activeTab === "members"
					? [
							{
								label: "Members",
							},
						]
					: props.activeTab === "roles"
						? [
								{
									label: "Roles",
								},
							]
						: []),
			]}
			subText="Manage your workspace settings, members, and roles."
			actions={() => (
				<>
					{props.actions?.()}
					{props.activeTab === "roles" && !location().pathname.includes("/new") && (
						<Link href="/workspace/roles/new" buttonVariant={ButtonVariant.Plain} external={false}>
							Create New Role
						</Link>
					)}
				</>
			)}
			bottomContent={() => (
				<div class="w-full text-white flex gap-4">
					<RouterLink
						to="/workspace"
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "general" ? "border-primary" : "border-transparent"}`}
					>
						General
					</RouterLink>

					<RouterLink
						to="/workspace/members"
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "members" ? "border-primary" : "border-transparent"}`}
					>
						Members
					</RouterLink>

					<RouterLink
						to="/workspace/roles"
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "roles" ? "border-primary" : "border-transparent"}`}
					>
						Roles
					</RouterLink>
				</div>
			)}
		/>
	);
};

export default WorkspaceHeader;
