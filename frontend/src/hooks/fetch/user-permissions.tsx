import { createQuery } from "@tanstack/solid-query";
import { GetCurrentPermissionsResponse, ListAllPermissionsResponse } from "~/bindings";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { userPermissionKeys } from "~/hooks/query-keys";
import { parsePermissionName, resourceTypes, userActionTypes } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import { ActionTypes, ResourceTypes, UserPermissionsT } from "~/utils/types";

type ParsedPermission = { resourceType: string; permission: string };

/**
 * Utility function to fetch all permissions for a workspace and map them
 * to their resourceType and action.
 */
export const getPermissions = async (wsId: string) => {
	const response = await httpRequest<ListAllPermissionsResponse>(
		`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
		{ method: "GET" }
	);

	if (!response.ok) {
		throw new Error("Failed to fetch permissions from server");
	}

	return response.data.permissions.map(
		(perm): Record<string, ParsedPermission> => ({
			[perm.id]: parsePermissionName(perm.name),
		})
	);
};

const useUserPermissionsQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: userPermissionKeys.current(wsId ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch user permissions" },
			staleTime: 5 * 60 * 1000,
			gcTime: 30 * 60 * 1000,
			queryFn: async (): Promise<UserPermissionsT> => {
				const [permissions, response] = await Promise.all([
					getPermissions(wsId!),
					httpRequest<GetCurrentPermissionsResponse>(
						`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/current-permissions`,
						{ method: "GET" }
					),
				]);

				let permissionsMap: Record<string, Record<string, string>> = {};
				for (const permObj of permissions) {
					for (const [permId, permDetail] of Object.entries(permObj)) {
						const { resourceType, permission } = permDetail;
						if (!permissionsMap[resourceType]) {
							permissionsMap[resourceType] = {};
						}
						permissionsMap[resourceType][permission] = permId;
					}
				}

				if (!response.ok) {
					throw new Error("Failed to fetch user permissions");
				}

				const userPermission = response.data;

				if (userPermission.type !== "member") {
					return userPermission as UserPermissionsT;
				}

				const validResourceTypes = new Set<string>(resourceTypes);
				const validActions = new Set<string>(userActionTypes);

				// @ts-expect-error just this once
				let detailedPermissions: Record<
					ResourceTypes,
					Record<ActionTypes, { permissionType: "include" | "exclude"; resources: Array<string> }>
				> & { type: "member" } = { type: "member" };

				for (const [resourceType, actionPermissions] of Object.entries(permissionsMap)) {
					if (resourceType === "type" || !validResourceTypes.has(resourceType)) continue;

					Object.entries(actionPermissions).forEach(([action, permId]) => {
						if (!validActions.has(action)) return;

						if (userPermission[permId]) {
							if (!detailedPermissions[resourceType as ResourceTypes]) {
								// @ts-expect-error TypeScript can't narrow string to resourceTypes after validation
								detailedPermissions[resourceType as ResourceTypes] = {};
							}
							detailedPermissions[resourceType as ResourceTypes][action as ActionTypes] =
								userPermission[permId];
						}
					});
				}

				return detailedPermissions;
			},
		};
	});
};

export default useUserPermissionsQuery;
