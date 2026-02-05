import { ActionTypes, MaybeAccessor, ResourceTypes, UserPermissionsT } from "~/utils/types";
import { useLastWorkspaceId } from "./state-hooks";
import { createMemo } from "solid-js";
import { get } from "~/utils/func";
import useFetchUserPermissions from "./use-fetch/use-fetch-user-permissions";

const useIsAllowed = (
	resourceType: ResourceTypes,
	action: ActionTypes,
	resId?: MaybeAccessor<string>,
	getResourceIds: boolean = false
) => {
	const [workspaceId] = useLastWorkspaceId();
	const [userPermissions] = useFetchUserPermissions();

	const isAllowed = createMemo(() => {
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
};

export default useIsAllowed;
