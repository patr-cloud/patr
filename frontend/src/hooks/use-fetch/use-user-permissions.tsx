import { createMemo, createResource } from "solid-js";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { AuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetCurrentPermissionsResponse, ListAllPermissionsResponse } from "~/bindings";
import { parsePermissionName, safelyParseJSON } from "~/utils/func";

export type ResourceTypes =
	| "billing"
	| "containerRegistryRepository"
	| "database"
	| "deployment"
	| "dnsRecord"
	| "domain"
	| "managedUrl"
	| "runner"
	| "secret"
	| "staticSite"
	| "volume"
	| "viewRoles"
	| "modifyRoles"
	| "editWorkspace";
export type ActionTypes =
	| "view"
	| "edit"
	| "makePayment"
	| "create"
	| "delete"
	| "push"
	| "pull"
	| "deleteImage"
	| "backup"
	| "restore"
	| "start"
	| "stop"
	| "add"
	| "verify"
	| "regenerateToken"
	| "upload";

export const resourceActionMap: Record<ResourceTypes, ActionTypes[]> = {
	billing: ["view", "edit", "makePayment"],
	containerRegistryRepository: ["create", "edit", "delete", "view", "push", "pull", "deleteImage"],
	database: ["view", "edit", "create", "delete", "backup", "restore"],
	deployment: ["view", "edit", "create", "delete", "start", "stop"],
	dnsRecord: ["view", "edit", "add", "delete"],
	domain: ["view", "add", "verify", "delete"],
	managedUrl: ["view", "edit", "delete", "add", "verify"],
	runner: ["view", "edit", "create", "delete", "regenerateToken"],
	secret: ["view", "edit", "create", "delete"],
	staticSite: ["view", "edit", "create", "delete", "upload", "start", "stop"],
	volume: ["create", "delete", "view", "edit"],
	viewRoles: [],
	modifyRoles: [],
	editWorkspace: [],
};

/**
 * Utility function to prevent redundant API calls by caching permissions in localStorage.
 * @param authState Current Authentication State
 * @param wsId Current Workspace ID
 * @returns Every Permission ID mapped to it's resourceType and action
 */
const getPermissions = async (authState: AuthState, wsId: string) => {
	console.log("[getPermissions] Called with:", { authType: authState?.type, wsId });

	if (!authState || authState.type !== "LoggedIn") {
		console.log("[getPermissions] User is not logged in, throwing error");
		throw new Error("User is not logged in");
	}

	const permissions =
		typeof window !== "undefined" && window.localStorage ? localStorage.getItem("user-permissions") : null;
	console.log("[getPermissions] localStorage permissions:", permissions ? "Found" : "Not found");

	let parsedPermissions = permissions ? safelyParseJSON<ListAllPermissionsResponse>(permissions) : undefined;
	console.log("[getPermissions] Parsed cached permissions:", parsedPermissions ? "Valid" : "Invalid/None");

	if (!parsedPermissions) {
		console.log("[getPermissions] Fetching permissions from API");
		const response = await httpRequest<ListAllPermissionsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
				},
			}
		);

		if (!response.ok) {
			console.error("[getPermissions] Failed to fetch permissions:", response.data.error);
			throw new Error("Failed to fetch permissions from server");
		}

		console.log("[getPermissions] Successfully fetched permissions from API");
		const permissionsData = response.data;
		if (typeof window !== "undefined" && window.localStorage) {
			localStorage.setItem("user-permissions", JSON.stringify(permissionsData));
			console.log("[getPermissions] Cached permissions to localStorage");
		}
		parsedPermissions = permissionsData as unknown as ListAllPermissionsResponse;
	}

	const mappedPermissions = parsedPermissions.permissions.map((perm) => {
		return { [perm.id]: parsePermissionName(perm.name) };
	});
	console.log("[getPermissions] Returning mapped permissions:", mappedPermissions);
	return mappedPermissions;
};

export type UserPermissionsT =
	| {
			type: "superAdmin";
	  }
	| ({
			type: "member";
	  } & Record<
			ResourceTypes,
			Record<ActionTypes, { permissionType: "include" | "exclude"; resources: Array<string> }>
	  >);

const useFetchUserPermissions = () => {
	console.log("[useFetchUserPermissions] Hook called");
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		const params = [authState(), workspaceId()] as const;
		console.log("[useFetchUserPermissions] fetchParams memo:", params);
		return params;
	});

	return createResource(fetchParams, async ([auth, wsId]) => {
		console.log("[useFetchUserPermissions] Resource fetcher called with:", { auth: auth?.type, wsId });

		if (!wsId || !auth || auth.type !== "LoggedIn") {
			console.log("[useFetchUserPermissions] Invalid auth or workspace, returning default member");
			return {
				type: "member" as const,
			};
		}

		// Check sessionStorage cache first
		const cachedPermissions =
			typeof window !== "undefined" && window.sessionStorage
				? sessionStorage.getItem(`user-permissions-${wsId}`)
				: null;
		console.log("[useFetchUserPermissions] Cached permissions:", cachedPermissions ? "Found" : "Not found");
		if (cachedPermissions) {
			const parsed = safelyParseJSON<UserPermissionsT>(cachedPermissions);
			if (parsed) {
				console.log("[useFetchUserPermissions] Using cached permissions from sessionStorage:", parsed);
				return parsed;
			}
		}

		try {
			console.log("[useFetchUserPermissions] Fetching permissions from API");
			const permissions = await getPermissions(auth, wsId);
			console.log("[useFetchUserPermissions] Fetched permissions:", permissions);

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

			console.log("[useFetchUserPermissions] Fetching current user permissions");
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
				const validResourceTypes = new Set<string>([
					"billing",
					"containerRegistryRepository",
					"database",
					"deployment",
					"dnsRecord",
					"domain",
					"managedUrl",
					"runner",
					"secret",
					"staticSite",
					"volume",
					"viewRoles",
					"modifyRoles",
					"editWorkspace",
				]);

				const validActions = new Set<string>([
					"view",
					"edit",
					"makePayment",
					"create",
					"delete",
					"push",
					"pull",
					"deleteImage",
					"backup",
					"restore",
					"start",
					"stop",
					"add",
					"verify",
					"regenerateToken",
					"upload",
				]);

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

							detailedPermissions[resourceType as ResourceTypes][action as ActionTypes] = userPermission[permId];
						} else {
							console.log(`[useFetchUserPermissions] No permission found for permId: ${permId}`);
						}
					});
				}

				// Cache to sessionStorage (workspace-specific)
				if (typeof window !== "undefined" && window.sessionStorage) {
					sessionStorage.setItem(`user-permissions-${wsId}`, JSON.stringify(detailedPermissions));
					console.log("[useFetchUserPermissions] Cached detailed permissions to sessionStorage");
				}
				console.log("[useFetchUserPermissions] Final detailed permissions:", detailedPermissions);

				return detailedPermissions;
			} else {
				console.log("[useFetchUserPermissions] User type is superAdmin, returning as-is");
				sessionStorage.setItem(`user-permissions-${wsId}`, JSON.stringify({ type: "superAdmin" }));
				return userPermission;
			}
		} catch (error) {
			console.error("[useFetchUserPermissions] Error fetching user permissions:", error);
			toast("Failed to load user information", "error");
			return {
				type: "member" as const,
			};
		}
	});
};

export default useFetchUserPermissions;
