import { useParams } from "@tanstack/solid-router";
import { Show } from "solid-js";
import { Initials, Table } from "~/components";
import { useRoleUsersQuery } from "~/hooks/fetch";

const UsersAssignedToRole = () => {
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	const usersQuery = useRoleUsersQuery(() => params().roleId);

	return (
		<div class="flex flex-col gap-4">
			<div class="flex items-center justify-between">
				<h3 class="text-lg text-white">Users with this role</h3>
				<span class="text-gray-400 text-sm">{usersQuery.data?.length ?? 0} users</span>
			</div>
			<Show
				when={(usersQuery.data ?? []).length > 0}
				fallback={<div class="text-gray-400 text-center py-8">No users have been assigned this role yet</div>}
			>
				<Table
					column_grids={["flex-1"]}
					headings={["Username"]}
					rows={usersQuery.data ?? []}
					renderRow={(item) => (
						<tr class="table-row">
							<td class="flex-1 flex items-center justify-center gap-2">
								<Initials firstName={item.firstName} lastName={item.lastName} size="xs" />
								<span class="truncate font-mono">{item.username}</span>
							</td>
						</tr>
					)}
				/>
			</Show>
		</div>
	);
};

export default UsersAssignedToRole;
