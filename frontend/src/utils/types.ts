import { Accessor } from "solid-js";
import { ErrorType } from "~/bindings";

export type SearchParams = Record<string, string | string[] | undefined>;
export type SetSearchParams = Record<
	string,
	string | string[] | number | number[] | boolean | boolean[] | null | undefined
>;
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
