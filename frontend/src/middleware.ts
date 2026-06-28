import { createMiddleware } from "@solidjs/start/middleware";
import type { RenewAccessTokenResponse } from "~/bindings";
import type { AuthState } from "~/hooks/state-hooks";

const AUTH_COOKIE = "authState";
const COOKIE_MAX_AGE = 60 * 60 * 24 * 7; // 7 days, matches the client's cookieStorage

function readCookie(request: Request, name: string): string | null {
	const header = request.headers.get("cookie");
	if (!header) return null;
	// Values are raw JSON (no `;`), matching cookieStorage's unencoded write.
	const match = header.match(new RegExp(`(?:^|; )${name}=([^;]+)`));
	return match ? match[1] : null;
}

// Set-Cookie matching cookieStorage's format: raw (unencoded) value so the
// client and the API read it back identically.
function authCookie(value: string, maxAge: number): string {
	return `${AUTH_COOKIE}=${value}; Max-Age=${maxAge}; Path=/; SameSite=Strict`;
}

// Decode a JWT's `exp` (seconds) without verifying the signature — we only need
// it to decide whether to refresh; the API still validates the token. Returns
// null if it can't be decoded (treated as expired).
function accessTokenExp(token: string): number | null {
	const payload = token.split(".")[1];
	if (!payload) return null;
	try {
		const b64 = payload
			.replace(/-/g, "+")
			.replace(/_/g, "/")
			.padEnd(Math.ceil(payload.length / 4) * 4, "=");
		const claims = JSON.parse(atob(b64)) as { exp?: number };
		return typeof claims.exp === "number" ? claims.exp : null;
	} catch {
		return null;
	}
}

/**
 * SSR auth handling. Runs per request before the page renders, so cookies and
 * redirects take effect *before* the response stream flushes — the render-time
 * `httpRequest` path can't do this (a `Set-Cookie` after the stream started
 * throws `ERR_HTTP_HEADERS_SENT`).
 *
 * When a logged-in request carries an expired access token, refresh it
 * server-side. Both outcomes are expressed as a returned `Response` (a redirect
 * with a `Set-Cookie`) — never by writing `event.response`, which is unreliable
 * from inside middleware (the underlying h3 response is already committed once
 * the handler resumes after an `await`):
 *  - refresh succeeds → re-issue the rotated `authState` and 302 back to the
 *    same URL; the re-request carries the fresh token, so this no-ops and the
 *    page renders normally (one extra round-trip, only when the token expired).
 *  - refresh fails → the session is dead: clear the cookie and 302 to /login,
 *    with no logged-out flash on protected routes.
 */
export default createMiddleware({
	onRequest: async (event) => {
		// Only top-level document navigations — skip assets and the /api proxy.
		if (!(event.request.headers.get("accept") ?? "").includes("text/html")) return;

		const raw = readCookie(event.request, AUTH_COOKIE);
		if (!raw) return;
		let auth: AuthState;
		try {
			auth = JSON.parse(raw) as AuthState;
		} catch {
			return;
		}
		if (!auth || auth.type !== "LoggedIn") return;

		// Still valid (with a small skew buffer) → let the render use it as-is.
		const exp = accessTokenExp(auth.accessToken);
		if (exp !== null && exp * 1000 > Date.now() + 10_000) return;

		// Expired (or undecodable): refresh server-side. Refresh tokens are
		// single-use and rotate, so the new pair is persisted via Set-Cookie.
		let next: AuthState | null = null;
		try {
			const resp = await fetch(`${import.meta.env.VITE_BASE_URL}/api/auth/access-token`, {
				method: "GET",
				headers: { Authorization: `Bearer ${auth.refreshToken}` },
			});
			if (resp.ok) {
				const data = (await resp.json()) as RenewAccessTokenResponse;
				next = { type: "LoggedIn", accessToken: data.accessToken, refreshToken: data.refreshToken };
			}
		} catch {
			// network error → treat as a failed refresh
		}

		const url = new URL(event.request.url);
		if (next) {
			return new Response(null, {
				status: 302,
				headers: {
					Location: url.pathname + url.search,
					"Set-Cookie": authCookie(JSON.stringify(next), COOKIE_MAX_AGE),
				},
			});
		}

		return new Response(null, {
			status: 302,
			headers: {
				Location: "/login",
				"Set-Cookie": authCookie("", 0),
			},
		});
	},
});
