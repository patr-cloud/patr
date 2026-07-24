import { test as base } from '@playwright/test';
import { type ApiClient, makeApiClient } from '@/helpers/api';
import { TURNSTILE_TOKEN } from '@/helpers/config';
import { randomIPv4 } from '@/helpers/ip';
import { DASHBOARD_URL } from '@/helpers/urls';

type Fixtures = {
	api: ApiClient;
};

export const test = base.extend<{}, Fixtures>({
	api: [
		async ({}, use) => {
			// app.patr.cloud server proxies /api/* to the same axum routes the
			// browser-side dashboard hits, so this URL mirrors real web traffic.
			await use(makeApiClient(`${DASHBOARD_URL}/api`));
		},
		{ scope: 'worker' },
	],
});

// Re-export expect for convenience in specs.
export { expect } from '@playwright/test';

// Per-test browser context with X-Real-IP injected on requests to our API
// only. Setting it via `extraHTTPHeaders` would send it on every request
// including cross-origin ones (fonts.gstatic.com, challenges.cloudflare.com),
// which trips CORS preflight failures and breaks Cloudflare Turnstile.
//
// Also bounds context.close() — React-Query background polls can keep a
// dashboard page busy and stall close indefinitely, eating the full per-test
// 60s timeout. Forcing a fast close keeps test runtime bounded.
export async function newContext(
	browser: import('@playwright/test').Browser,
	clientIp = randomIPv4(),
) {
	const context = await browser.newContext();

	// Only route /api/** — every routed request round-trips through Playwright's
	// IPC, and a page load pulls many module/asset requests. Routing them all
	// starves Playwright's internal scheduler and makes
	// page.waitForTimeout/expect-polling take 60s instead of ms.
	await context.route(`${DASHBOARD_URL}/api/**`, async (route) => {
		const headers = { ...route.request().headers(), 'x-real-ip': clientIp };
		await route.continue({ headers });
	});

	// Stub the Cloudflare Turnstile widget so tests don't depend on the external
	// challenges.cloudflare.com script. The auth submit buttons are gated on a
	// Turnstile token; with the production frontend build the page is interactive
	// instantly, so under parallel workers the real async CF script can land after
	// the test already checked the button — leaving it stuck disabled. Block that
	// script and provide a stub that fires the always-passes test token at once
	// (and again on reset, for re-verify flows). The backend accepts it verbatim.
	await context.route('https://challenges.cloudflare.com/**', (route) => route.abort());

	// Block the external Google Fonts pulled in by `app.css`
	// (@import fonts.googleapis.com → fonts.gstatic.com). A pending @import is
	// render- and load-blocking, so on a CI runner with slow/throttled external
	// network the request stalls and the document `load` event never fires —
	// `page.goto` (waitUntil:'load' by default) then hangs the full 60s test
	// timeout. Aborting them makes navigation depend only on the local stack;
	// pages fall back to system fonts, which tests don't assert on.
	await context.route('https://fonts.googleapis.com/**', (route) => route.abort());
	await context.route('https://fonts.gstatic.com/**', (route) => route.abort());

	await context.addInitScript((token: string) => {
		const state: { callback: ((t: string) => void) | null } = { callback: null };
		(window as unknown as { turnstile: unknown }).turnstile = {
			render: (_container: unknown, options: { callback?: (t: string) => void }) => {
				state.callback = options?.callback ?? null;
				options?.callback?.(token);
				return 'stub-widget';
			},
			reset: () => state.callback?.(token),
			remove: () => {},
		};
	}, TURNSTILE_TOKEN);

	// Bound context.close() (as the comment above promises). React-Query
	// background polls keep a dashboard page busy, so the native close can stall
	// indefinitely and eat the 60s test timeout — e.g. after creating the first
	// workspace swaps in the dashboard. Closing the pages first stops that activity so
	// the native close returns immediately; the race is a backstop, and since the
	// pages are already closed a timed-out context is inert (no polls left to leak).
	const nativeClose = context.close.bind(context);
	context.close = (async () => {
		await Promise.race([
			(async () => {
				await Promise.all(context.pages().map((page) => page.close().catch(() => {})));
				// Swallow "already closed": after a test timeout Playwright disposes
				// the context itself, and a throwing double-close from a finally block
				// would REPLACE the real failure in the report with a teardown stack
				// pointing here.
				await nativeClose().catch(() => {});
			})(),
			new Promise<void>((resolve) => setTimeout(resolve, 5_000)),
		]);
	}) as typeof context.close;

	return context;
}
