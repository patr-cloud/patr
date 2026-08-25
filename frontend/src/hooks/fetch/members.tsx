import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetUserDetailsResponse } from "~/bindings/GetUserDetailsResponse";
import { ListUsersInWorkspaceResponse } from "~/bindings/ListUsersInWorkspaceResponse";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { memberKeys, userInfoKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export type WorkspaceMember = {
	userId: string;
	firstName: string;
	lastName: string;
	fullName: string;
	username: string;
	roleIds: string[];
	/**
	 * True for the synthetic row representing the workspace's super-admin.
	 * The list endpoint omits the owner; the UI prepends a synthesized row
	 * from `useWorkspaceOwnerQuery` and marks it `isOwner` so Edit/Remove
	 * controls can be hidden.
	 */
	isOwner: boolean;
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

				const userDetailsPromises: Promise<WorkspaceMember | null>[] = Object.keys(response.data.users).map(
					async (userId) => {
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
							roleIds: (response.data.users[userId] || []).map((grant) => grant.roleId),
							isOwner: false,
						};
					}
				);

				const members = (await Promise.all(userDetailsPromises)).filter(
					(m): m is WorkspaceMember => m !== null
				);

				return { members, totalCount };
			},
		};
	});
};

/**
 * Fetch the workspace owner's basic info. Returned as a [WorkspaceMember]
 * with `isOwner: true` so callers can prepend it to the members list. The
 * `useMembersQuery` no longer fetches workspace info itself — callers pass
 * the already-known `superAdminId` from `useWorkspaceInfoQuery`, which
 * avoids doubling the network round-trips on every workspace switch.
 */
export const useWorkspaceOwnerQuery = (superAdminId: Accessor<string | undefined>) => {
	const [authState] = useAuthState();

	return createQuery<WorkspaceMember | null>(() => {
		const auth = authState();
		const ownerId = superAdminId();
		return {
			queryKey: userInfoKeys.byId(ownerId ?? ""),
			enabled: !!ownerId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch workspace owner" },
			queryFn: async () => {
				if (!ownerId) return null;
				const resp = await httpRequest<GetUserDetailsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/${ownerId}`,
					{ method: "GET" }
				);
				if (!resp.ok) return null;
				const firstName = resp.data.firstName || "";
				const lastName = resp.data.lastName || "";
				return {
					userId: resp.data.id || ownerId,
					firstName,
					lastName,
					fullName: `${firstName} ${lastName}`,
					username: resp.data.username || "",
					roleIds: [],
					isOwner: true,
				};
			},
		};
	});
};
