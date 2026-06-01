import { RenewAccessTokenResponse } from "~/bindings";
import { ErrorResponse, FetchResult } from "./types";
import { getRequestEvent, isServer } from "solid-js/web";
import { cookieStorage } from "@solid-primitives/storage";
import type { AuthState } from "~/hooks/state-hooks";

// Module-level dedup: when many requests hit 401 in parallel (common on page
// load), they used to each fire their own /auth/access-token call. The first
// one rotates the refresh token, the rest send the now-stale one and fail.
// Hold a single in-flight promise and let concurrent callers share it.
let inFlightRefresh: Promise<RenewAccessTokenResponse | null> | null = null;

async function refreshAccessToken(): Promise<RenewAccessTokenResponse | null> {
	// Refresh tokens are single-use and rotate on every renew. Doing this on
	// the server during SSR would burn the token before the browser ever
	// sees it — the SSR can't propagate the new cookie back via Set-Cookie
	// reliably from inside a SolidJS data fetch. Skip on server; the browser
	// will re-fire the failed request on hydration and refresh from there.
	if (isServer) return null;
	if (inFlightRefresh) return inFlightRefresh;
	inFlightRefresh = (async () => {
		const currentAuthState = cookieStorage.getItem("authState");
		if (!currentAuthState) return null;
		const authState = JSON.parse(currentAuthState) as AuthState | null;
		if (!authState || authState.type !== "LoggedIn") return null;

		const refreshResp = await fetch(`${import.meta.env.VITE_BASE_URL}/api/auth/access-token`, {
			method: "GET",
			headers: { Authorization: `Bearer ${authState.refreshToken}` },
		});

		if (!refreshResp.ok) return null;

		const refreshData = (await refreshResp.json()) as RenewAccessTokenResponse;

		cookieStorage.setItem(
			"authState",
			JSON.stringify({
				...authState,
				accessToken: refreshData.accessToken,
				refreshToken: refreshData.refreshToken,
			}),
			{
				expires: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7),
				path: "/",
				sameSite: "Strict",
			}
		);

		if (!isServer) {
			sessionStorage.clear();
		}

		return refreshData;
	})();
	try {
		return await inFlightRefresh;
	} finally {
		inFlightRefresh = null;
	}
}

/**
 * A wrapper around the Fetch API, adds a few things, such as:
 * - Default headers, including Content-Type application/json
 * - JSON response parsing
 * - Error handling
 * - Type safety with generics
 *
 * @param url {string} The URL of the request
 * @param options {RequestInit} The options for the request
 * @returns {Promise<FetchResult<T>>} Returns a promise that resolves to a FetchResult<T>, if the request succeeds,
 * then `resp.data` will be of type `T` else it will be of type [`ErrorResponse`](./types.ts)
 */
const httpRequest = async <T>(url: string, options?: RequestInit): Promise<FetchResult<T>> => {
	try {
		const event = getRequestEvent();
		const resp = await fetch(url, {
			method: "GET",
			credentials: "include",
			...options,
			headers: {
				"Content-Type": "application/json",
				...(options?.headers || {}),
				...(isServer && event?.request?.headers?.get("cookie")
					? { Cookie: event.request.headers.get("cookie")! }
					: {}),
			},
		});

		// Handle empty responses (204 No Content, etc.)
		const contentType = resp.headers.get("content-type");
		const hasJsonContent = contentType?.includes("application/json");

		let data;
		if (hasJsonContent && resp.body) {
			data = await resp.json();
		} else {
			data = {};
		}

		if (resp.ok) {
			return {
				data: data as T,
				headers: resp.headers,
				ok: resp.ok,
				status: resp.status,
				statusText: resp.statusText,
			};
		}

		const errorData = data as ErrorResponse;
		const defaultErrorReturn = {
			data: data as ErrorResponse,
			headers: resp.headers,
			ok: resp.ok,
			status: resp.status,
			statusText: resp.statusText,
		};

		if (errorData.error === "malformedAccessToken") {
			console.log("Access token malformed, redirecting to login...", data);
			cookieStorage.removeItem("authState");
			if (!isServer) {
				window.location.href = "/login";
			}
			return defaultErrorReturn;
		}

		if (errorData.error === "authorizationTokenInvalid") {
			const refreshData = await refreshAccessToken();
			if (!refreshData) {
				cookieStorage.removeItem("authState");
				if (!isServer) {
					window.location.href = "/login";
				}
				return defaultErrorReturn;
			}

			// Retry the original request with the new access token
			const retryResp = await fetch(url, {
				...options,
				headers: {
					...(options?.headers || {}),
					Authorization: `Bearer ${refreshData.accessToken}`,
				},
			});

			const retryData = hasJsonContent ? await retryResp.json() : {};

			if (retryResp.ok) {
				return {
					data: retryData as T,
					headers: retryResp.headers,
					ok: true,
					status: retryResp.status,
					statusText: retryResp.statusText,
				};
			} else {
				return {
					data: retryData as ErrorResponse,
					headers: retryResp.headers,
					ok: false,
					status: retryResp.status,
					statusText: retryResp.statusText,
				};
			}
		}
		return {
			data: data as ErrorResponse,
			headers: resp.headers,
			ok: resp.ok,
			status: resp.status,
			statusText: resp.statusText,
		};
	} catch (error) {
		console.error("Fetch error:", error);

		// Return a proper error response structure for network errors
		return {
			data: {
				error: error instanceof Error ? error.message : "Network request failed",
			} as ErrorResponse,
			headers: new Headers(),
			ok: false,
			status: 0,
			statusText: "Network Error",
		};
	}
};

export { httpRequest };
