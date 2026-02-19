import { ActionTypes, MaybeAccessor, ResourceTypes, UserPermissionsT } from "~/utils/types";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { createMemo } from "solid-js";
import { get, resourceActionMap } from "~/utils/func";
import { useFetchPermissions, useFetchUserPermissions } from "~/hooks/fetch";
import { useIsMounted } from "~/hooks";
import { getPermissions } from "./fetch/user-permissions";

type ResourceActionMapType = typeof resourceActionMap;
type ActionsForResource<T extends ResourceTypes> = ResourceActionMapType[T][number];

/**
 * Custom hook to check what actions a user has for a specific resource.
 */
const useGetPermissions = <T extends ResourceTypes>(resourceType: T, resId: MaybeAccessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const [userPermissions] = useFetchUserPermissions();
	const isMounted = useIsMounted();

	const permissions = createMemo(() => {
		const allFalse = {} as Record<ActionsForResource<T>, boolean>;
		resourceActionMap[resourceType].forEach((action) => {
			allFalse[action as ActionsForResource<T>] = false;
		});

		if (!isMounted()) return allFalse;

		const actionTypes = resourceActionMap[resourceType];
		const userPerms = userPermissions() as unknown as UserPermissionsT | null;
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

		const userPermissionsOnResource = userPerms[resourceType] as Record<
			ActionTypes,
			{ permissionType: "include" | "exclude"; resources: string[] }
		>;
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

			if (actionPermission.permissionType === "exclude") {
				permissions[action as ActionsForResource<T>] = !actionPermission.resources.includes(resourceId);
			} else if (actionPermission.permissionType === "include") {
				permissions[action as ActionsForResource<T>] = actionPermission.resources.includes(resourceId);
			} else {
				permissions[action as ActionsForResource<T>] = false;
			}
		});

		console.log(permissions);

		return permissions;
	});

	console.log("[useGetPermissions] finishing with:", permissions());
	return permissions;
};

const useIsAllowed = (resourceType: ResourceTypes, action: ActionTypes, resId?: MaybeAccessor<string>) => {
	const [workspaceId] = useLastWorkspaceId();
	const [userPermissions] = useFetchUserPermissions();
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
		const permissions = userPermissions() as unknown as UserPermissionsT | null;

		if (!permissions) return false;
		if (!wsId) return false;

		if (permissions.type === "superAdmin") {
			return true;
		}

		const resourcePermissions = permissions[resourceType];

		if (!resourcePermissions) return false;
		const actionPermission = resourcePermissions[action];

		if (!actionPermission) return false;

		if (actionPermission.permissionType === "exclude") {
			if (actionPermission.resources.length === 0) {
				// Exclude nothing means allow all
				return true;
			}

			if (resourceId && !actionPermission.resources.includes(resourceId)) {
				// Resource ID is not in the excluded list, so allowed, this is only if resourceId is provided
				return true;
			}
		}

		if (actionPermission.permissionType === "include") {
			// Include nothing means allow none
			if (actionPermission.resources.length === 0) {
				return false;
			}

			if (resourceId && actionPermission.resources.includes(resourceId)) {
				// Resource ID is in the included list, so allowed, this is only if resourceId is provided
				return true;
			}
		}

		return false;
	});

	return isAllowed;
};

export { useGetPermissions };
export default useIsAllowed;
