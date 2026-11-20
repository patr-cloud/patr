import { createMemo } from "solid-js";
import { usePermissionsQuery, useUserPermissionsQuery } from "~/hooks/fetch";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { parsePermissionName, resourceTypes, userActionTypes, workspaceLevelResourceTypes } from "~/utils/func";
import { ActionTypes, ResourceTypes, UserPermissionsT } from "~/utils/types";
import useIsMounted from "./is-mounted";

/** Shown when the API refuses a role binding with `grantExceedsPermissions`. */
const GRANT_CEILING_MESSAGE =
	"You can only grant roles whose permissions you hold yourself. Remove the role you added, or narrow it to resources you have access to, and try again.";

/** Shown when the API refuses a role's new permissions with `grantExceedsPermissions`. */
const ROLE_CEILING_MESSAGE =
	"You can only add permissions you hold across the whole workspace. Untick the ones you don't hold and try again.";

/**
 * What the current user may give out. Nobody can grant a permission they don't
 * hold themselves, and the API refuses a request that tries.
 *
 * Every answer is `true` until the user's permissions have loaded, for a super
 * admin, and for a permission this dashboard has no entry for, so nothing is
 * disabled on a guess. The API stays the authority.
 */
const useGrantCeiling = () => {
	const [workspaceId] = useLastWorkspaceId();
	const userPermissionsQuery = useUserPermissionsQuery();
	const permissionsQuery = usePermissionsQuery(() => workspaceId() ?? "");
	const isMounted = useIsMounted();

	/** Where the user holds each permission, by permission ID. Undefined when nothing limits them. */
	const held = createMemo(() => {
		// Checked through status first so a cold query doesn't suspend the form.
		if (!isMounted() || !userPermissionsQuery.isSuccess || !permissionsQuery.isSuccess) return undefined;
		const userPermissions = userPermissionsQuery.data as unknown as UserPermissionsT;
		if (userPermissions.type === "superAdmin") return undefined;

		const scopes = new Map<string, string[]>();
		for (const permission of permissionsQuery.data.permissions) {
			const { resourceType, permission: action } = parsePermissionName(permission.name);
			if (!resourceTypes.includes(resourceType)) continue;
			if (!workspaceLevelResourceTypes.has(resourceType) && !userActionTypes.includes(action)) continue;
			scopes.set(permission.id, userPermissions[resourceType as ResourceTypes]?.[action as ActionTypes] ?? []);
		}
		return scopes;
	});

	/** Whether the user holds a permission on a resource. The workspace's own ID covers every resource. */
	const holds = (permissionId: string, resourceId: string) => {
		const scopes = held()?.get(permissionId);
		if (!scopes) return true;
		return scopes.includes(workspaceId() ?? "") || scopes.includes(resourceId);
	};

	return {
		/** Whether anything limits what the user can grant. False for a super admin. */
		isLimited: () => held() !== undefined,
		/** Whether the user holds a permission across the whole workspace, which adding it to a role needs. */
		holdsEverywhere: (permissionId: string) => holds(permissionId, workspaceId() ?? ""),
		/** Whether the user holds every one of a role's permissions on every resource it would be bound at. */
		canGrant: (permissionIds: string[], resourceIds: string[]) =>
			resourceIds.every((resourceId) => permissionIds.every((permissionId) => holds(permissionId, resourceId))),
	};
};

export { GRANT_CEILING_MESSAGE, ROLE_CEILING_MESSAGE };
export default useGrantCeiling;
