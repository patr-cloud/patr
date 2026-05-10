import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
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
	roleIds: string[];
};

export const useMembersQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: memberKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch members" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListUsersInWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				const totalCount = Number(response.headers.get("x-total-count") ?? 0);

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
					const id = user.id || "";

					return {
						userId: id,
						firstName,
						lastName,
						fullName: `${firstName} ${lastName}`,
						roleIds: response.data.users[userId] || [],
					} satisfies WorkspaceMember;
				});

				const members = (await Promise.all(userDetailsPromises)).filter(
					(m): m is WorkspaceMember => m !== null
				);

				return { members, totalCount };
			},
		};
	});
};
