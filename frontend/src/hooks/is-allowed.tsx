import { ActionTypes, MaybeAccessor, ResourceTypes, UserPermissionsT } from "~/utils/types";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { createMemo } from "solid-js";
import { get, isWorkspaceScoped, resourceActionMap, workspaceLevelResourceTypes } from "~/utils/func";
import { useUserPermissionsQuery } from "~/hooks/fetch";
import { useIsMounted } from "~/hooks";

type ResourceActionMapType = typeof resourceActionMap;
type ActionsForResource<T extends ResourceTypes> = ResourceActionMapType[T][number];

/**
 * Custom hook to check what actions a user has for a specific resource.
 */
const useGetPermissions = <T extends ResourceTypes>(resourceType: T, resId: MaybeAccessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const userPermissionsQuery = useUserPermissionsQuery();
	const isMounted = useIsMounted();

	const permissions = createMemo(() => {
		const allFalse = {} as Record<ActionsForResource<T>, boolean>;
		resourceActionMap[resourceType].forEach((action) => {
			allFalse[action as ActionsForResource<T>] = false;
		});

		if (!isMounted()) return allFalse;

		const actionTypes = resourceActionMap[resourceType];
		const userPerms = userPermissionsQuery.data as unknown as UserPermissionsT | null;
		const resourceId = get(resId);
		const wsId = get(workspaceId);
		const auth = authState();

		if (!userPerms) return allFalse;
		if (userPerms.type === "superAdmin") {
			// Super admins have all permissions
			const allPermissions = {} as Record<ActionsForResource<T>, boolean>;
			actionTypes.forEach((action) => {
				allPermissions[action as ActionsForResource<T>] = true;
			});
			return allPermissions;
		}
		if (!resourceId) return allFalse;
		if (!wsId) return allFalse;
		if (!auth || auth.type !== "LoggedIn") return allFalse;

		const userPermissionsOnResource = userPerms[resourceType] as Record<ActionTypes, Array<string>>;
		if (!userPermissionsOnResource) return allFalse;

		const permissions = {} as Record<ActionsForResource<T>, boolean>;

		console.log(actionTypes);
		actionTypes.forEach((action) => {
			const actionPermission = userPermissionsOnResource[action];
			if (!actionPermission) {
				permissions[action as ActionsForResource<T>] = false;
				console.log(`[useGetPermissions memo] No permission entry for action ${action}, defaulting to false`);
				return;
			}

			// A scope is a resource id; the workspace id is the root and
			// covers every resource under it.
			permissions[action as ActionsForResource<T>] =
				actionPermission.includes(wsId) || actionPermission.includes(resourceId);
		});

		console.log(permissions);

		return permissions;
	});

	return permissions;
};

const useIsAllowed = (resourceType: ResourceTypes, action: ActionTypes, resId?: MaybeAccessor<string>) => {
	const [workspaceId] = useLastWorkspaceId();
	const userPermissionsQuery = useUserPermissionsQuery();
	const isMounted = useIsMounted();
	console.log("[useIsAllowed] Initializing with:", {
		resourceType,
		action,
		resId: get(resId),
	});

	const isAllowed = createMemo(() => {
		// Prevent SSR hydration mismatches by always returning false until mounted
		if (!isMounted()) return false;

		const resourceId = get(resId);
		const wsId = get(workspaceId);
		const permissions = userPermissionsQuery.data as unknown as UserPermissionsT | null;

		if (!permissions) return false;
		if (!wsId) return false;

		if (permissions.type === "superAdmin") {
			return true;
		}

		const resourcePermissions = permissions[resourceType];

		if (!resourcePermissions) return false;

		// For workspace-level resource types (e.g. viewRoles, modifyRoles),
		// the permission is stored under key "" — remap the action for lookup
		const lookupAction = workspaceLevelResourceTypes.has(resourceType) ? ("" as ActionTypes) : action;
		const actionPermission = resourcePermissions[lookupAction];

		if (!actionPermission) return false;

		// Workspace-scoped actions (e.g. deployment::create, billing::view) are not
		// dependent on a specific resource ID — just check that the entry exists
		if (workspaceLevelResourceTypes.has(resourceType) || isWorkspaceScoped(resourceType, action)) {
			return true;
		}

		// For resource-dependent actions without a resourceId, the caller is asking
		// "does the user have this capability at all?" — return true if the entry exists
		if (!resourceId) {
			return true;
		}

		// Resource-dependent actions with a resourceId: a grant at the workspace
		// root covers everything, otherwise the scope must name the resource
		return actionPermission.includes(wsId) || actionPermission.includes(resourceId);
	});

	return isAllowed;
};

export { useGetPermissions };
export default useIsAllowed;
