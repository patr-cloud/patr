import { A, useNavigate, useLocation } from "@solidjs/router";
import { ButtonVariant, Link, PageContainerHead } from "~/components";

interface WorkspaceHeaderProps {
	workspaceName?: string;
	activeTab: "general" | "members" | "roles";
}

const WorkspaceHeader = (props: WorkspaceHeaderProps) => {
	const navigate = useNavigate();
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
			actions={() =>
				props.activeTab === "roles" &&
				!location.pathname.includes("/new") && (
					<Link href="/workspace/roles/new" buttonVariant={ButtonVariant.Plain} external={false}>
						Create New Role
					</Link>
				)
			}
			bottomContent={() => (
				<div class="w-full text-white flex gap-4">
					<A
						href="/workspace"
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "general" ? "border-primary" : "border-transparent"}`}
					>
						General
					</A>

					<A
						href="/workspace/members"
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "members" ? "border-primary" : "border-transparent"}`}
					>
						Members
					</A>

					<A
						href="/workspace/roles"
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "roles" ? "border-primary" : "border-transparent"}`}
					>
						Roles
					</A>
				</div>
			)}
		/>
	);
};

export default WorkspaceHeader;
