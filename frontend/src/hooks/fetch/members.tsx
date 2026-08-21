import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListUsersInWorkspaceResponse } from "~/bindings/ListUsersInWorkspaceResponse";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { memberKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export type WorkspaceMember = {
	userId: string;
	firstName: string;
	lastName: string;
	fullName: string;
	email: string;
	roleIds: string[];
	/**
	 * True for the workspace's super-admin. They hold access directly on the
	 * workspace rather than through a role, so the UI hides the Edit/Remove
	 * controls for them.
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

				const members: WorkspaceMember[] = response.data.users.map((user) => ({
					userId: user.id,
					firstName: user.firstName,
					lastName: user.lastName,
					fullName: `${user.firstName} ${user.lastName}`.trim(),
					email: user.email,
					roleIds: user.roleIds,
					isOwner: user.isOwner,
				}));

				return { members, totalCount };
			},
		};
	});
};
