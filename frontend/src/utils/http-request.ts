import {
  LoginRequest,
  LoginResponse,
  RenewAccessTokenResponse,
} from "~/bindings";
import { ErrorResponse, FetchResult } from "./types";
import { getRequestEvent } from "solid-js/web";
import { cookieStorage } from "@solid-primitives/storage";
import type { AuthState } from "~/hooks/state-hooks";

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
const httpRequest = async <T>(
  url: string,
  options?: RequestInit
): Promise<FetchResult<T>> => {
  try {
    const event = getRequestEvent();
    const resp = await fetch(url, {
      method: "GET",
      credentials: "include",
      ...options,
      headers: {
        "Content-Type": "application/json",
        ...(options?.headers || {}),
        ...(event?.request.headers
          ? Object.fromEntries(event.request.headers)
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
      window.location.href = "/login";
      return defaultErrorReturn;
    }

    if (errorData.error === "authorizationTokenInvalid") {
      const currentAuthState = cookieStorage.getItem("authState");
      if (!currentAuthState) {
        return defaultErrorReturn;
      }

      const authState = JSON.parse(
        cookieStorage.getItem("authState") || "null"
      ) as AuthState | null;

      if (!authState || authState.type !== "LoggedIn") {
        window.location.href = "/login";
        return defaultErrorReturn;
      }

      const refreshResp = await fetch(
        `${import.meta.env.VITE_BASE_URL}/api/auth/access-token`,
        {
          method: "GET",
          headers: {
            Authorization: `Bearer ${authState.refreshToken}`,
          },
        }
      );

      if (!refreshResp.ok) {
        cookieStorage.removeItem("authState");
        window.location.href = "/login";
        return defaultErrorReturn;
      }

      const refreshData =
        (await refreshResp.json()) as RenewAccessTokenResponse;

      cookieStorage.setItem(
        "authState",
        JSON.stringify({
          ...authState,
          accessToken: refreshData.accessToken,
        }),
        {
          expires: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7), // 7 days
          path: "/",
          sameSite: "Strict",
        }
      );

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
        error:
          error instanceof Error ? error.message : "Network request failed",
      } as ErrorResponse,
      headers: new Headers(),
      ok: false,
      status: 0,
      statusText: "Network Error",
    };
  }
};

interface EndpointMap {
  Login: {
    method: "POST";
    path: string;
    body: LoginRequest;
    response: LoginResponse;
  };
}

const endpointConfig: {
  [K in keyof EndpointMap]: {
    method: EndpointMap[K]["method"];
    path: EndpointMap[K]["path"];
  };
} = {
  Login: {
    method: "POST",
    path: `${import.meta.env.VITE_BASE_URL}/api/auth/sign-in`,
  },
};

type Key = keyof EndpointMap;
type RequestBody<K extends Key> = EndpointMap[K] extends { body: infer B }
  ? B
  : undefined;
type PathParams<K extends Key> = EndpointMap[K] extends { pathParams: infer P }
  ? P
  : undefined;
type QueryParams<K extends Key> = EndpointMap[K] extends {
  queryParams: infer Q;
}
  ? Q
  : undefined;

type ResponseType<K extends Key> = EndpointMap[K] extends { response: infer R }
  ? R
  : never;

/// utility wrapper
/**
 * @deprecated Use [httpRequest](./http-request.ts) instead
 */
async function makeRequest<K extends Key>(
  key: K,
  options: {
    body: RequestBody<K>;
    pathParams?: PathParams<K>;
    queryParams?: QueryParams<K>;
    headers?: Record<string, string>;
  }
): Promise<ResponseType<K>> {
  const cfg = endpointConfig[key];

  let path = cfg.path as string;

  // Build path by substituting pathParams into cfg.path
  if (options.pathParams) {
    for (const [paramKey, paramValue] of Object.entries(options.pathParams)) {
      path = path.replace(
        `:${paramKey}`,
        encodeURIComponent(String(paramValue))
      );
    }
  }

  // Build query string if needed
  if (options.queryParams) {
    const qp = new URLSearchParams(
      Object.entries(options.queryParams).map(([k, v]) => [k, String(v)])
    ).toString();
    path += `?${qp}`;
  }

  // Method
  const method = cfg.method;

  const fetchOptions: RequestInit = {
    method,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
  };

  const resp = await fetch(path, fetchOptions);
  if (!resp.ok) {
    throw new Error(`HTTP error! status: ${resp.status}`);
  }

  return (await resp.json()) as ResponseType<K>;
}

export { httpRequest, makeRequest };
