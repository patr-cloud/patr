import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
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

export const useAllRolesQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<ListAllRolesResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: roleKeys.list(wsId ?? "", undefined, undefined),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const response = await httpRequest<ListAllRolesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role`,
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
