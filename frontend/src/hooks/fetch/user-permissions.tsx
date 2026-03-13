import { makePersisted } from "@solid-primitives/storage";
import { createMemo, createResource, createSignal } from "solid-js";
import { GetCurrentPermissionsResponse, ListAllPermissionsResponse } from "~/bindings";
import { useToast } from "~/components";
import { AuthState, useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { parsePermissionName, resourceTypes, safelyParseJSON, userActionTypes } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import { ActionTypes, ResourceTypes, UserPermissionsT } from "~/utils/types";

/**
 * Utility function to prevent redundant API calls by caching permissions in localStorage.
 * @param authState Current Authentication State
 * @param wsId Current Workspace ID
 * @returns Every Permission ID mapped to it's resourceType and action
 */
export const getPermissions = async (authState: AuthState, wsId: string) => {
	console.log("[getPermissions] Called with:", { authType: authState?.type, wsId });

	if (!authState || authState.type !== "LoggedIn") {
		console.log("[getPermissions] User is not logged in, throwing error");
		throw new Error("User is not logged in");
	}

	const isServer = typeof window === "undefined";
	const [permissions, setPermissions] = isServer
		? createSignal<string | null>(null)
		: makePersisted(createSignal<string | null>(null), {
				name: `user-permissions`,
			});

	let parsedPermissions: ListAllPermissionsResponse | undefined = undefined;

	if (!isServer && permissions()) {
		parsedPermissions = safelyParseJSON<ListAllPermissionsResponse>(permissions() || "");
	}

	if (!parsedPermissions) {
		console.log("[getPermissions] Fetching permissions from API");
		const response = await httpRequest<ListAllPermissionsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("[getPermissions] Failed to fetch permissions:", response.data.error);
			throw new Error("Failed to fetch permissions from server");
		}

		console.log("[getPermissions] Successfully fetched permissions from API");
		const permissionsData = response.data;

		setPermissions(JSON.stringify(permissionsData));
		parsedPermissions = permissionsData as unknown as ListAllPermissionsResponse;
	}

	const mappedPermissions = parsedPermissions.permissions.map((perm) => {
		return { [perm.id]: parsePermissionName(perm.name) };
	});

	console.log("[getPermissions] Returning mapped permissions:", mappedPermissions);
	return mappedPermissions;
};

const useUserPermissions = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		const params = [authState(), workspaceId()] as const;
		return params;
	});

	const permissions = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			console.log("[useFetchUserPermissions] Invalid auth or workspace, returning default member");
			return {
				type: "member" as const,
			};
		}

		const isServer = typeof window === "undefined";
		const [cachedPermissions, setCachedPermissions] = isServer
			? createSignal<string | null>(null)
			: makePersisted(createSignal<string | null>(null), {
					storage: sessionStorage,
					name: `user-permissions-${wsId}`,
				});
		console.log("[useFetchUserPermissions] Cached permissions:", cachedPermissions() ? "Found" : "Not found");
		if (!isServer && cachedPermissions()) {
			const parsed = safelyParseJSON<UserPermissionsT>(cachedPermissions() || "");
			if (parsed) {
				console.log("[useFetchUserPermissions] Using cached permissions from sessionStorage:", parsed);
				return parsed;
			}
		}

		try {
			console.log("[useFetchUserPermissions] Fetching permissions from API");
			const permissions = await getPermissions(auth, wsId);

			// Transform permissions array to { [resourceType]: { [action]: permissionId } }
			let permissionsMap: Record<string, Record<string, string>> = {};

			for (const permObj of permissions) {
				for (const [permId, permDetail] of Object.entries(permObj)) {
					const { resourceType, action } = permDetail;

					if (!permissionsMap[resourceType]) {
						permissionsMap[resourceType] = {};
					}

					permissionsMap[resourceType][action] = permId;
				}
			}

			console.log("[useFetchUserPermissions] Transformed permissionsMap:", permissionsMap);

			const response = await httpRequest<GetCurrentPermissionsResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/current-permissions`,
				{
					method: "GET",
				}
			);

			if (!response.ok) {
				console.error("[useFetchUserPermissions] Failed to fetch user permissions:", response);
				toast("Failed to load user information", "error");
				return {
					type: "member" as const,
				};
			}

			const userPermission = response.data;
			console.log("[useFetchUserPermissions] User permission response:", userPermission);

			if (userPermission.type === "member") {
				console.log("[useFetchUserPermissions] User type is member, building detailed permissions");
				const validResourceTypes = new Set<string>(resourceTypes);
				const validActions = new Set<string>(userActionTypes);

				// @ts-expect-error just this once
				let detailedPermissions: Record<
					ResourceTypes,
					Record<ActionTypes, { permissionType: "include" | "exclude"; resources: Array<string> }>
				> & { type: "member" } = { type: "member" };

				for (const [resourceType, actionPermissions] of Object.entries(permissionsMap)) {
					if (resourceType === "type" || !validResourceTypes.has(resourceType)) {
						console.log(`[useFetchUserPermissions] Skipping invalid resourceType: ${resourceType}`);
						continue;
					}

					Object.entries(actionPermissions).forEach(([action, permId]) => {
						if (!validActions.has(action)) {
							console.log(`[useFetchUserPermissions] Skipping invalid action: ${action}`);
							return;
						}

						if (userPermission[permId]) {
							console.log(
								`[useFetchUserPermissions] Adding permission for ${resourceType}.${action}:`,
								userPermission[permId]
							);
							if (!detailedPermissions[resourceType as ResourceTypes]) {
								// @ts-expect-error TypeScript can't narrow string to resourceTypes after validation
								detailedPermissions[resourceType as ResourceTypes] = {};
							}

							detailedPermissions[resourceType as ResourceTypes][action as ActionTypes] =
								userPermission[permId];
						} else {
							console.log(`[useFetchUserPermissions] No permission found for permId: ${permId}`);
						}
					});
				}

				setCachedPermissions(JSON.stringify(detailedPermissions));
				console.log("[useFetchUserPermissions] Returning detailed permissions:", detailedPermissions);
				return detailedPermissions;
			} else {
				console.log("[useFetchUserPermissions] User type is superAdmin, returning as is");
				setCachedPermissions(JSON.stringify(userPermission));

				return userPermission;
			}
		} catch (error) {
			console.error("Error fetching user permissions:", error);
			toast("Failed to load user permissions", "error");

			return {
				type: "member" as const,
			};
		}
	});

	return permissions;
};

export default useUserPermissions;
