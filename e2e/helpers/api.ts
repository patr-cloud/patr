import { randomIPv4 } from '@/helpers/ip';
import { USER_AGENT } from '@/helpers/config';

export type ApiClient = {
  baseUrl: string;
  request: <T = unknown>(
    method: string,
    path: string,
    opts?: {
      body?: unknown;
      headers?: Record<string, string>;
      token?: string;
      clientIp?: string;
    },
  ) => Promise<T>;
};

export function makeApiClient(baseUrl: string): ApiClient {
  return {
    baseUrl,
    async request<T>(
      method: string,
      path: string,
      opts: {
        body?: unknown;
        headers?: Record<string, string>;
        token?: string;
        clientIp?: string;
      } = {},
    ): Promise<T> {
      const headers: Record<string, string> = {
        'X-Real-IP': opts.clientIp ?? randomIPv4(),
        'User-Agent': USER_AGENT,
        ...(opts.body !== undefined ? { 'Content-Type': 'application/json' } : {}),
        ...(opts.token ? { Authorization: `Bearer ${opts.token}` } : {}),
        ...opts.headers,
      };

      const res = await fetch(`${baseUrl}${path}`, {
        method,
        headers,
        body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
      });

      const text = await res.text();
      if (!res.ok) {
        throw new Error(`API ${method} ${path} → ${res.status}: ${text.slice(0, 500)}`);
      }
      return (text ? JSON.parse(text) : undefined) as T;
    },
  };
}
