import { A, useNavigate, useLocation } from "@solidjs/router";
import { ButtonVariant, Link, PageContainerHead } from "~/components";

interface WorkspaceHeaderProps {
	workspaceName?: string;
	activeTab: "workspace" | "roles";
}

const WorkspaceHeader = (props: WorkspaceHeaderProps) => {
	const navigate = useNavigate();
	const location = useLocation();

	return (
		<PageContainerHead
			breadcrumbs={[
				{
					label: "Workspace Settings",
					url: props.activeTab === "roles" ? "/workspace" : undefined,
				},
				...(props.activeTab === "roles"
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
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "workspace" ? "border-primary" : "border-transparent"}`}
					>
						Manage Workspace
					</A>

					<A
						href="/workspace/roles"
						onClick={() => navigate("/workspace/roles")}
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "roles" ? "border-primary" : "border-transparent"}`}
					>
						Manage Roles
					</A>
				</div>
			)}
		/>
	);
};

export default WorkspaceHeader;
