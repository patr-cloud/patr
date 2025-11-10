import { LoginRequest, LoginResponse } from "~/bindings";
import { ErrorResponse, FetchResult } from "./types";

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
const doFetch = async <T>(
  url: string,
  options?: RequestInit
): Promise<FetchResult<T>> => {
  try {
    const resp = await fetch(url, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
      ...options,
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

    console.log(data);

    if (!resp.ok) {
      console.error(`HTTP error! status: ${resp.status}`);
      return {
        data: data as ErrorResponse,
        headers: resp.headers,
        ok: resp.ok,
        status: resp.status,
        statusText: resp.statusText,
      };
    }

    return {
      data: data as T,
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
 * @deprecated Use doFetch instead
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

export { doFetch, makeRequest };
