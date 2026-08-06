import { createQuery } from "@tanstack/solid-query";
import { ListWorkspaceInvitesResponse } from "~/bindings/ListWorkspaceInvitesResponse";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { inviteKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

/**
 * Fetch the pending invites for the current workspace. Requires the caller to
 * have the "view roles" permission (same as the member list).
 */
export const useInvitesQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: inviteKeys.list(wsId ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch invites" },
			queryFn: async () => {
				const response = await httpRequest<ListWorkspaceInvitesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/user/invite`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data.invites;
			},
		};
	});
};
