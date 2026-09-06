import { createQuery, keepPreviousData } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetRoleInfoResponse } from "~/bindings/GetRoleInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { ListUsersForRoleResponse } from "~/bindings/ListUsersForRoleResponse";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { roleKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useRolesQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: roleKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch roles" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListAllRolesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					roles: response.data.roles,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

export const useAllRolesQuery = (
	page: Accessor<string | undefined>,
	count: Accessor<string | undefined>,
	// Defaults to the active workspace; pass one to fetch another workspace's
	// roles (the token screens list grants across all of the user's workspaces).
	workspaceId?: Accessor<string | undefined>
) => {
	const [authState] = useAuthState();
	const [lastWorkspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId ? workspaceId() : lastWorkspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: roleKeys.allRoles(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch roles" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListAllRolesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					roles: response.data.roles,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

export const useRoleInfoQuery = (roleId: Accessor<string>, workspaceId?: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [lastWorkspaceId] = useLastWorkspaceId();

	return createQuery<GetRoleInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId ? workspaceId() : lastWorkspaceId();
		const id = roleId();
		return {
			queryKey: roleKeys.detail(wsId ?? "", id),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			// Switching the role a binding grants re-keys this query. Without a
			// placeholder that re-suspends, and the whole editor blanks mid-edit.
			placeholderData: keepPreviousData,
			meta: { errorMessage: "Failed to fetch role info" },
			queryFn: async () => {
				const response = await httpRequest<GetRoleInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role/${id}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};

export const useRoleUsersQuery = (roleId: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = roleId();
		return {
			queryKey: roleKeys.users(wsId ?? "", id),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			meta: { errorMessage: "Failed to fetch users for role" },
			queryFn: async () => {
				const response = await httpRequest<ListUsersForRoleResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role/${id}/users`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data.users || [];
			},
		};
	});
};
