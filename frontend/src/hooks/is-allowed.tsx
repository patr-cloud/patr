import { ActionTypes, MaybeAccessor, ResourceTypes, UserPermissionsT } from "~/utils/types";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { createMemo } from "solid-js";
import { get } from "~/utils/func";
import { useFetchUserPermissions } from "~/hooks/fetch";
import { useIsMounted } from "~/hooks";

/**
 * Custom hook to check what actions a user has for a specific resource.
 */
const useGetPermissions = async (resourceType: ResourceTypes, resId: MaybeAccessor<string>) => {
	const [workspaceId] = useLastWorkspaceId();
	const [userPermissions] = useFetchUserPermissions();
	const isMounted = useIsMounted();

	console.log("[useGetPermissions] Initializing with:", {
		resourceType,
		resId: get(resId),
	});

	const permissions = createMemo(() => {
		// Prevent SSR hydration mismatches by always returning null until mounted
		if (!isMounted()) return null;

		const resourceId = get(resId);
		const wsId = get(workspaceId);
		const permissions = userPermissions() as unknown as UserPermissionsT | null;

		if (!permissions) return null;
		if (!wsId) return null;

		if (permissions.type === "superAdmin") {
			return { permissionType: "include", resources: [] };
		}

		const resourcePermissions = permissions[resourceType];

		if (!resourcePermissions) return null;

		// return resourcePermissions[action];
	});

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
				console.log("[useIsAllowed memo] Exclude none = allow all, returning true");
				return true;
			}

			if (resourceId && !actionPermission.resources.includes(resourceId)) {
				// Resource ID is not in the excluded list, so allowed, this is only if resourceId is provided
				console.log("[useIsAllowed memo] Resource not in exclude list, returning true");
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

			console.log("[useIsAllowed memo] Include check: resource not in include list or no specific resource");
		}

		console.log("[useIsAllowed memo] Defaulting to false");
		return false;
	});

	return isAllowed;
};

export default useIsAllowed;
