import { createInfiniteQuery, createQueries, createQuery, keepPreviousData } from "@tanstack/solid-query";
import { Accessor, createMemo, createSignal } from "solid-js";
import { Role, WithId } from "~/bindings";
import { GetRoleInfoResponse } from "~/bindings/GetRoleInfoResponse";
import { ListAllRolesResponse } from "~/bindings/ListAllRolesResponse";
import { ListRoleBindingsResponse } from "~/bindings/ListRoleBindingsResponse";

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

/** How many roles a picker loads per page as it's scrolled. */
const ROLE_PICKER_PAGE_SIZE = 20;

/** One page of roles, with the total so the next page can be worked out. */
type RolePage = {
	roles: WithId<Role>[];
	totalCount: number;
	page: number;
};

/**
 * Loads a workspace's roles a page at a time, for pickers that load the next
 * page as they're scrolled. [search] narrows them by name on the server.
 */
const useRolesInfiniteQuery = (search: Accessor<string>, workspaceId?: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [lastWorkspaceId] = useLastWorkspaceId();

	return createInfiniteQuery(() => {
		const auth = authState();
		const wsId = workspaceId ? workspaceId() : lastWorkspaceId();
		const s = search();
		const searchParam = s ? `&search[name]=${encodeURIComponent(s)}` : "";
		return {
			queryKey: roleKeys.infiniteList(wsId ?? "", s),
			// A new search re-keys this. Keep the previous results up until the
			// new ones land, rather than blanking the open picker.
			placeholderData: keepPreviousData,
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch roles" },
			initialPageParam: 0,
			queryFn: async ({ pageParam }: { pageParam: number }): Promise<RolePage> => {
				const response = await httpRequest<ListAllRolesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role?page=${pageParam}&count=${ROLE_PICKER_PAGE_SIZE}${searchParam}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					roles: response.data.roles,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
					page: pageParam,
				};
			},
			getNextPageParam: (lastPage: RolePage): number | undefined => {
				const loaded = (lastPage.page + 1) * ROLE_PICKER_PAGE_SIZE;
				return loaded < lastPage.totalCount ? lastPage.page + 1 : undefined;
			},
		};
	});
};

/**
 * A paged, searchable role dropdown for [BindingRows].
 *
 * Bound roles may not be paged in yet, so their names are looked up one by
 * one; a role seen on any page or search keeps its name after that.
 */
export const useRolePicker = (boundRoleIds: Accessor<string[]>, workspaceId?: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [lastWorkspaceId] = useLastWorkspaceId();
	const wsId = () => (workspaceId ? workspaceId() : lastWorkspaceId());

	const [search, setSearch] = createSignal("");
	const rolesQuery = useRolesInfiniteQuery(search, workspaceId);

	const options = createMemo(() =>
		(rolesQuery.data?.pages ?? []).flatMap((page) =>
			page.roles.map((role) => ({ label: role.name, value: role.id }))
		)
	);

	const seenNames = createMemo((previous: Map<string, string>) => {
		const next = new Map(previous);
		for (const option of options()) next.set(option.value, option.label);
		return next;
	}, new Map<string, string>());

	const unseenRoles = createQueries(() => {
		const auth = authState();
		const id = wsId();
		return {
			queries: boundRoleIds()
				.filter((roleId) => !seenNames().has(roleId))
				.map((roleId) => ({
					queryKey: roleKeys.detail(id ?? "", roleId),
					enabled: !!id && !!auth && auth.type === "LoggedIn",
					queryFn: () => fetchRoleInfo(id!, roleId),
				})),
		};
	});

	/** A role's name, or its id until the name loads. */
	const label = (roleId: string) =>
		seenNames().get(roleId) ?? unseenRoles.find((query) => query.data?.id === roleId)?.data?.name ?? roleId;

	return {
		options,
		label,
		onSearch: setSearch,
		onLoadMore: () => {
			if (rolesQuery.hasNextPage && !rolesQuery.isFetchingNextPage) rolesQuery.fetchNextPage();
		},
		isLoadingMore: () => rolesQuery.isFetchingNextPage || rolesQuery.isPlaceholderData,
	};
};

/** Fetches one role, for [useRoleInfoQuery] and [useRolePicker]'s name lookups. */
const fetchRoleInfo = async (workspaceId: string, roleId: string) => {
	const response = await httpRequest<GetRoleInfoResponse>(
		`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/role/${roleId}`,
		{ method: "GET" }
	);

	if (!response.ok) {
		throw new Error(response.data.error);
	}

	return response.data;
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
			queryFn: () => fetchRoleInfo(wsId!, id),
		};
	});
};

export const useRoleBindingsQuery = (roleId: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = roleId();
		return {
			queryKey: roleKeys.bindings(wsId ?? "", id),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			meta: { errorMessage: "Failed to fetch the role's bindings" },
			queryFn: async () => {
				const response = await httpRequest<ListRoleBindingsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/role/${id}/bindings`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data.bindings || [];
			},
		};
	});
};
