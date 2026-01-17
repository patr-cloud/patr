import { createMemo } from "solid-js";
import { get } from "~/utils/func";
import { useLastWorkspaceId } from "../state-hooks";
import { ActionTypes, ResourceTypes, UserPermissionsT } from "./use-user-permissions";
import { MaybeAccessor } from "~/utils/types";
import { resourceActionMap } from "./use-user-permissions";

type ResourceActionMapType = typeof resourceActionMap;
type ActionsForResource<T extends ResourceTypes> = ResourceActionMapType[T][number];

export const useResourceIdPermissions = <T extends ResourceTypes>(resourceType: T, resId: MaybeAccessor<string>) => {
	// return an object of all action types possible in this resource type with boolean values indicating if allowed or not
	// I also want this to be type safe, so that only valid action types for the resource type are included
	console.log("[useResourceIdPermissions] Called with resourceType:", resourceType);
	const actionTypes = resourceActionMap[resourceType];
	const [workspaceId] = useLastWorkspaceId();

	return createMemo(() => {
		console.log("[useResourceIdPermissions memo] Computing permissions for resourceType:", resourceType);
		const permissions = {} as Record<ActionsForResource<T>, boolean>;
		const resourceId = get(resId);
		console.log("[useResourceIdPermissions memo] resourceId:", resourceId, "workspaceId:", workspaceId());

		actionTypes.forEach((action) => {
			console.log("[useResourceIdPermissions memo] Calling useIsAllowed for action:", action);
			const [isAllowed] = useIsAllowed(resourceType, action as ActionTypes, () => resourceId);
			console.log("[useResourceIdPermissions memo] Result for action", action, ":", isAllowed);
			permissions[action as ActionsForResource<T>] = isAllowed;
		});

		console.log("[useResourceIdPermissions memo] Final permissions:", permissions);
		return permissions;
	});
};

const useIsAllowed = (
	resourceType: ResourceTypes,
	action: ActionTypes,
	resId?: MaybeAccessor<string>,
	getResourceIds: boolean = false
) => {
	console.log("[useIsAllowed] Called with resourceType:", resourceType, "action:", action);
	const [workspaceId] = useLastWorkspaceId();

	const isAllowed = createMemo(() => {
		console.log("[useIsAllowed memo] Computing for", resourceType, action);
		const resourceId = get(resId);
		console.log("[useIsAllowed memo] resourceId:", resourceId, "workspaceId:", workspaceId());

		if (!workspaceId()) {
			console.log("[useIsAllowed memo] No workspace ID, returning false");
			return false;
		}

		if (typeof window === "undefined" || !window.sessionStorage) {
			console.log("[useIsAllowed memo] Window or sessionStorage not available, returning false");
			return false;
		}

		const userPermissions = sessionStorage.getItem(`user-permissions-${workspaceId()}`);
		if (!userPermissions) {
			console.log("[useIsAllowed memo] No permissions in storage, returning false");
			return false;
		}

		const parsedPermissions = JSON.parse(userPermissions) as UserPermissionsT;
		console.log("[useIsAllowed memo] Parsed permissions type:", parsedPermissions.type);

		if (parsedPermissions.type === "superAdmin") {
			// Super admins have all permissions
			console.log("[useIsAllowed memo] Super admin, returning true");
			return true;
		}

		const resourcePermissions = parsedPermissions[resourceType];

		if (!resourcePermissions) {
			// no such resource type found
			console.log("[useIsAllowed memo] No resource permissions for", resourceType, ", returning false");
			return false;
		}

		const actionPermission = resourcePermissions[action];

		if (!actionPermission) {
			console.log("[useIsAllowed memo] No action permission for", action, ", returning false");
			return false;
		}

		console.log("[useIsAllowed memo] Action permission:", actionPermission);

		if (actionPermission.permissionType === "exclude") {
			if (actionPermission.resources.length === 0) {
				// Exclude nothing means allow all
				console.log("[useIsAllowed memo] Exclude none = allow all, returning true");
				return true;
			}

			if (resourceId && !actionPermission.resources.includes(resourceId)) {
				// Resource ID is not in the excluded list, so allowed, this is only if resourceId is provided
				console.log("[useIsAllowed memo] Resource not in exclude list, returning true");
				return true;
			}

			// If getResourceIds is true, we still return true since there are excludes
			if (getResourceIds) {
				console.log("[useIsAllowed memo] getResourceIds=true for exclude, returning true");
				return true;
			}
			console.log("[useIsAllowed memo] Exclude check: resource in exclude list or no specific resource");
		}

		if (actionPermission.permissionType === "include") {
			// Include nothing means allow none
			if (actionPermission.resources.length === 0) {
				console.log("[useIsAllowed memo] Include none = deny all, returning false");
				return false;
			}

			if (resourceId && actionPermission.resources.includes(resourceId)) {
				// Resource ID is in the included list, so allowed, this is only if resourceId is provided
				console.log("[useIsAllowed memo] Resource in include list, returning true");
				return true;
			}

			// If getResourceIds is true, we still return true if there are any includes
			if (getResourceIds) {
				console.log("[useIsAllowed memo] getResourceIds=true for include, returning true");
				return true;
			}
			console.log("[useIsAllowed memo] Include check: resource not in include list or no specific resource");
		}

		console.log("[useIsAllowed memo] Defaulting to false");
		return false;
	});

	const permissionDetails = createMemo<
		| {
				permissionType: "include" | "exclude";
				resources: string[];
		  }
		| undefined
	>(() => {
		console.log("[permissionDetails memo] Computing for", resourceType, action);
		if (!workspaceId() || typeof window === "undefined" || !window.sessionStorage) {
			console.log("[permissionDetails memo] No workspace or window, returning undefined");
			return undefined;
		}

		const userPermissions = sessionStorage.getItem(`user-permissions-${workspaceId()}`);
		if (!userPermissions) {
			console.log("[permissionDetails memo] No permissions in storage, returning undefined");
			return undefined;
		}

		const parsedPermissions = JSON.parse(userPermissions) as UserPermissionsT;

		if (parsedPermissions.type === "superAdmin") {
			console.log("[permissionDetails memo] Super admin, returning undefined");
			return undefined;
		}

		const resourcePermissions = parsedPermissions[resourceType];
		if (!resourcePermissions) {
			console.log("[permissionDetails memo] No resource permissions, returning undefined");
			return undefined;
		}

		const actionPermission = resourcePermissions[action];
		if (!actionPermission || !getResourceIds) {
			console.log("[permissionDetails memo] No action permission or getResourceIds=false, returning undefined");
			return undefined;
		}

		console.log("[permissionDetails memo] Returning action permission:", actionPermission);
		return actionPermission;
	});

	const result = [isAllowed(), permissionDetails()] as const;
	console.log("[useIsAllowed] Returning result:", result);
	return result;
};

export default useIsAllowed;
