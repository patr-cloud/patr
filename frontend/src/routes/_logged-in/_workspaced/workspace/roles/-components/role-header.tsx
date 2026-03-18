import { Link, useParams } from "@tanstack/solid-router";
import { PageContainerHead } from "~/components";

interface RoleHeaderProps {
	roleName?: string;
	workspaceName?: string;
	activeTab: "users" | "permissions";
}

const RoleHeader = (props: RoleHeaderProps) => {
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	return (
		<PageContainerHead
			breadcrumbs={[
				{
					label: "Workspace Settings",
					url: "/workspace",
				},
				{
					label: "Roles",
					url: "/workspace/roles",
				},
				{
					label: props.roleName || "Loading...",
				},
			]}
			subText={`Manage the ${props.roleName || ""} role settings, permissions, and assigned users.`}
			bottomContent={() => (
				<div class="w-full text-white flex gap-4">
					<Link
						to="/workspace/roles/$roleId"
						params={{ roleId: params().roleId }}
						search={{ tab: "" }}
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "permissions" ? "border-primary" : "border-transparent"}`}
					>
						Edit Permissions
					</Link>

					<Link
						to="/workspace/roles/$roleId"
						params={{ roleId: params().roleId }}
						search={{ tab: "users" }}
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "users" ? "border-primary" : "border-transparent"}`}
					>
						Users
					</Link>
				</div>
			)}
		/>
	);
};

export default RoleHeader;
