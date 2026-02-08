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
			title="Manage Role"
			titleUrl="/workspace-settings/roles"
			subTitle={props.roleName || "Loading..."}
			bottomContent={() => (
				<div class="w-full text-white flex gap-4">
					<A
						href={`/workspace-settings/roles/${params.roleId}`}
						class={`pb-2 px-2 border-b-2 ${props.activeTab === "permissions" ? "border-primary" : "border-transparent"}`}
					>
						Edit Permissions
					</A>

					<A
						href={`/workspace-settings/roles/${params.roleId}?tab=users`}
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
