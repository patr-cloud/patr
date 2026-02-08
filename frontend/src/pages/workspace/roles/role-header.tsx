import { A, useParams } from "@solidjs/router";
import { PageContainerHead } from "~/components";

interface RoleHeaderProps {
	roleName?: string;
	workspaceName?: string;
	activeTab: "users" | "permissions";
}

const RoleHeader = (props: RoleHeaderProps) => {
	const params = useParams();

	return (
		<PageContainerHead
			breadcrumbs={[
				{
					label: "Workspace",
					url: "/",
				},
				{
					label: props.roleName || "Loading...",
				},
			]}
			subText={`Manage the ${props.roleName || ""} role settings, permissions, and assigned users.`}
			bottomContent={() => (
				<div class="w-full text-white flex gap-4">
					<A
						href={`/workspace/roles/${params.roleId}`}
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "permissions" ? "border-primary" : "border-transparent"}`}
					>
						Edit Permissions
					</A>

					<A
						href={`/workspace/roles/${params.roleId}?tab=users`}
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "users" ? "border-primary" : "border-transparent"}`}
					>
						Users
					</A>
				</div>
			)}
		/>
	);
};

export default RoleHeader;
