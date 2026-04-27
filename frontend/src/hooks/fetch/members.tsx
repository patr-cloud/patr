import { createQuery } from "@tanstack/solid-query";
import { GetUserDetailsResponse } from "~/bindings/GetUserDetailsResponse";
import { ListUsersInWorkspaceResponse } from "~/bindings/ListUsersInWorkspaceResponse";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { memberKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export type WorkspaceMember = {
	userId: string;
	firstName: string;
	lastName: string;
	fullName: string;
	username: string;
	roleIds: string[];
};

// Backend paginates over (user, role) pairs, but the UI merges all roles per
// user into one row. To paginate over the merged list on the frontend, fetch
// all rows in one shot and slice client-side.
const FETCH_ALL_PAGE_SIZE = 1000;

export const useMembersQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: memberKeys.list(wsId ?? "", "0", String(FETCH_ALL_PAGE_SIZE)),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch members" },
			queryFn: async () => {
				const response = await httpRequest<ListUsersInWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user?page=0&count=${FETCH_ALL_PAGE_SIZE}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				const userDetailsPromises = Object.keys(response.data.users).map(async (userId) => {
					const userResponse = await httpRequest<GetUserDetailsResponse>(
						`${import.meta.env.VITE_BASE_URL}/api/user/${userId}`,
						{ method: "GET" }
					);

					if (!userResponse.ok) {
						return null;
					}

					const user = userResponse.data;
					const firstName = user.firstName || "";
					const lastName = user.lastName || "";
					const username = user.username || "";
					const id = user.id || "";

					return {
						userId: id,
						firstName,
						lastName,
						fullName: `${firstName} ${lastName}`,
						username,
						roleIds: response.data.users[userId] || [],
					} satisfies WorkspaceMember;
				});

				const members = (await Promise.all(userDetailsPromises)).filter(
					(m): m is WorkspaceMember => m !== null
				);

				return { members, totalCount: members.length };
			},
		};
	});
};
