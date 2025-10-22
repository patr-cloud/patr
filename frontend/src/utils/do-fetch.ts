import { LoginRequest, LoginResponse } from "~/bindings";

const doFetch = async <T>(url: string, options?: RequestInit) => {
  const resp = await fetch(url, {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
    },
    ...options,
  });

  if (!resp.ok) {
    throw new Error(`HTTP error! status: ${resp.status}`);
  }

  const data = (await resp.json()) as T;

  return {
    data,
    headers: resp.headers,
    ok: resp.ok,
    status: resp.status,
    statusText: resp.statusText,
  };
};

interface EndpointMap {
  Login: {
    method: "POST";
    path: "http://localhost:3001/api/auth/sign-in";
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
    path: "http://localhost:3001/api/auth/sign-in",
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

  return (await resp.json()) as Promise<ResponseType<K>>;
}

export { doFetch };
export default makeRequest;
