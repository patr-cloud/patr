import { Accessor } from "solid-js";
import { ErrorType } from "~/bindings";

export type MaybeAccessor<T> = T | Accessor<T>;
export type EventT<T, E> = T & { currentTarget: E };
export type ErrorResponse = {
	message: string;
	success: false;
	error: ErrorType;
};

/**
 * A discriminated union type representing the result of a fetch operation.
 *
 * The `ok` field acts as the discriminant to determine success or failure:
 * - When `ok: false`, the `data` field contains an `ErrorResponse`
 * - When `ok: true`, the `data` field contains the expected success type `T`
 *
 * @template T - The type of data returned on successful fetch
 *
 * @example
 * ```ts
 * const result: FetchResult<User> = await fetchUser();
 * if (result.ok) {
 *   console.log(result.data.name); // TypeScript knows data is User
 * } else {
 *   console.error(result.data.error); // TypeScript knows data is ErrorResponse
 * }
 * ```
 */
export type FetchResult<T> =
	| {
			ok: false;
			status: number;
			statusText: string;
			headers: Headers;
			data: ErrorResponse;
	  }
	| {
			ok: true;
			status: number;
			statusText: string;
			headers: Headers;
			data: T;
	  };

export type ResourceTypes =
	| "billing"
	| "containerRegistryRepository"
	| "database"
	| "deployment"
	| "dnsRecord"
	| "domain"
	| "managedURL"
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

// Local copies of wire shapes whose generated bindings went away with the
// flattened role DTOs. The remaining consumers (the include/exclude role
// matrix and the token permission editor) are retired by the role-editor
// and token-screen reworks later in the stack.
export type ResourcePermissionType =
	{ permissionType: "include"; resources: Array<string> } | { permissionType: "exclude"; resources: Array<string> };

/** The pre-flattening role metadata shape, still used by the roles list. */
export type Role = { name: string; description?: string };
