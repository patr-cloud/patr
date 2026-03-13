import { useParams } from "@solidjs/router";
import { createResource, Show, Suspense } from "solid-js";
import { GetUserDetailsResponse } from "~/bindings/GetUserDetailsResponse";
import { ListUsersForRoleResponse } from "~/bindings/ListUsersForRoleResponse";
import { Initials, Table, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const UsersAssignedToRole = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const params = useParams();
	const toast = useToast();

	const [usersWithDetails] = createResource(
		() => [authState(), workspaceId(), params.roleId] as const,
		async ([auth, workspaceId, roleId]) => {
			if (!auth || auth.type !== "LoggedIn" || !workspaceId || !roleId) {
				return [];
			}

			const response = await httpRequest<ListUsersForRoleResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${roleId}/users`,
				{
					method: "GET",
				}
			);

			console.log("Users for role response:", response.data);
			if (!response.ok) {
				console.error("Failed to fetch users for role:", response.data.error);
				toast("Failed to fetch users for role", "error");
				return [];
			}

			const userDetailsPromises = (response.data.users || []).map(async (userId) => {
				const userResponse = await httpRequest<GetUserDetailsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/${userId}`,
					{
						method: "GET",
					}
				);

				if (userResponse.ok) {
					return userResponse.data;
				} else {
					console.error(`Failed to fetch details for user ${userId}:`, userResponse.data.error);
					return null;
				}
			});

			return (await Promise.all(userDetailsPromises)).filter((user) => user !== null);
		}
	);

	return (
		<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading users...</div>}>
			<div class="flex flex-col gap-4">
				<div class="flex items-center justify-between">
					<h3 class="text-lg text-white">Users with this role</h3>
					<span class="text-gray-400 text-sm">{usersWithDetails()?.length} users</span>
				</div>
				<Show
					when={(usersWithDetails() ?? []).length > 0}
					fallback={
						<div class="text-gray-400 text-center py-8">No users have been assigned this role yet</div>
					}
				>
					<Table
						column_grids={["flex-1"]}
						headings={["Username"]}
						rows={usersWithDetails() ?? []}
						renderRow={(item) => (
							<tr class="table-row">
								<td class="flex-2 flex items-center justify-center gap-2">
									<Initials firstName={item.firstName} lastName={item.lastName} size="xs" />
									<span class="truncate font-mono">{item.username}</span>
								</td>
							</tr>
						)}
					/>
				</Show>
			</div>
		</Suspense>
	);
};

export default UsersAssignedToRole;
