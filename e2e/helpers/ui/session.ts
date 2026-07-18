import type { BrowserContext } from '@playwright/test';
import type { UserHandle } from '@/helpers/user';
import { DASHBOARD_URL } from '@/helpers/urls';

// Programmatic login: seed the same cookies the SPA would set after a real
// /login flow. Skips the Turnstile + form round-trip for specs that aren't
// testing login itself.
//
// frontend/src/hooks/state-hooks.tsx — `authState` and `lastWorkspaceId` are
// both persisted via @solid-primitives/storage's cookieStorage, which
// JSON-encodes the value. Cookies must be set on the SAME origin Playwright
// loads the SPA from (playwright.config baseURL = DASHBOARD_URL via the
// Caddy proxy), not on the Vinxi dev port. Cookies bound to a different
// origin are not sent on navigation.

export async function loginAs(
	context: BrowserContext,
	user: UserHandle,
	opts: { workspaceId?: string } = {},
): Promise<void> {
	const authState = JSON.stringify({
		type: 'LoggedIn',
		accessToken: user.accessToken,
		refreshToken: user.refreshToken,
	});
	// Playwright: provide `url` (which derives domain+path) OR `domain`+`path`,
	// not both. Use url-form to keep the helper short.
	//
	// The API reads `authState` directly via `headers::Cookie::get("authState")`
	// then `serde_json::from_str` — i.e. expects RAW JSON, not URI-encoded
	// (api/src/utils/layers/web_dashboard_auth_cookie_layer.rs:103). The
	// frontend's cookieStorage matches: it only URI-encodes `&` and `;`. The
	// values we set contain neither, so writing raw JSON is correct for both
	// server and client readers.
	const cookies = [
		{
			name: 'authState',
			value: authState,
			url: DASHBOARD_URL,
			sameSite: 'Strict' as const,
		},
	];
	if (opts.workspaceId) {
		cookies.push({
			name: 'lastWorkspaceId',
			value: JSON.stringify(opts.workspaceId),
			url: DASHBOARD_URL,
			sameSite: 'Strict' as const,
		});
	}
	await context.addCookies(cookies);
}
