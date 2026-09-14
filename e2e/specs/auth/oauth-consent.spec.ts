import { test, expect, newContext, expectUrl, loginAs } from '@/prelude';
import { createUserAccount } from '@/helpers/user';
import { startAuthorization, E2E_REDIRECT_URI } from '@/helpers/oauth';
import { openLoginPage, fillLoginForm, submitLogin } from '@/helpers/ui/login';

// The consent screen sits at the root of the route tree, next to
// accept-invite, because it has to render in both auth states — neither
// `_logged-in` nor `_logged-out` can host it. That makes every assertion here
// double as a routing assertion: none of this copy exists on any other page,
// so a regression in the route tree fails these outright.
//
// The client's redirect URI points at a port nothing listens on. Each spec
// intercepts that one URL rather than the whole tree — a blanket `/**` route
// starves the dev server's HMR socket (see e2e/CLAUDE.md).

const REDIRECT_GLOB = 'http://localhost:19999/**';

async function stubRedirectTarget(page: import('@playwright/test').Page) {
	await page.route(REDIRECT_GLOB, (route) =>
		route.fulfill({ status: 200, contentType: 'text/plain', body: 'ok' }),
	);
}

test.describe('oauth > consent [UI]', () => {
	test('shows what the app is asking for, and returns a code on approval', async ({
		browser,
		api,
	}) => {
		const user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user);
		const page = await context.newPage();

		try {
			await stubRedirectTarget(page);
			const { requestId, state } = await startAuthorization();

			await page.goto(`/authorize?requestId=${requestId}`, { waitUntil: 'domcontentloaded' });

			await expect(page.getByText(/E2E Test App wants to access your Patr account/i)).toBeVisible({
				timeout: 15_000,
			});
			// The honest wording about a token acting with the user's full
			// access matters more than the scope list — if that line ever
			// quietly disappears, the screen is misrepresenting the grant.
			await expect(page.getByText(/act on your behalf with the same access you have/i)).toBeVisible();

			await page.getByRole('button', { name: /Authorize E2E Test App/i }).click();

			await expectUrl(page, /localhost:19999\/cb/);
			const landed = new URL(page.url());
			expect(landed.searchParams.get('code')).toBeTruthy();
			expect(landed.searchParams.get('state')).toBe(state);
		} finally {
			await context.close();
		}
	});

	test('cancelling returns access_denied rather than hanging', async ({ browser, api }) => {
		const user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user);
		const page = await context.newPage();

		try {
			await stubRedirectTarget(page);
			const { requestId, state } = await startAuthorization();

			await page.goto(`/authorize?requestId=${requestId}`, { waitUntil: 'domcontentloaded' });
			await page.getByRole('button', { name: /^Cancel$/ }).click();

			await expectUrl(page, /localhost:19999\/cb/);
			const landed = new URL(page.url());
			expect(landed.searchParams.get('error')).toBe('access_denied');
			expect(landed.searchParams.get('state')).toBe(state);
			expect(landed.searchParams.get('code')).toBeNull();
		} finally {
			await context.close();
		}
	});

	test('an already-decided request reports itself as expired', async ({ browser, api }) => {
		const user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user);
		const page = await context.newPage();

		try {
			await stubRedirectTarget(page);
			const { requestId } = await startAuthorization();

			await page.goto(`/authorize?requestId=${requestId}`, { waitUntil: 'domcontentloaded' });
			await page.getByRole('button', { name: /Authorize E2E Test App/i }).click();
			await expectUrl(page, /localhost:19999\/cb/);

			// Re-presenting a consumed request must not produce a second code.
			await page.goto(`/authorize?requestId=${requestId}`, { waitUntil: 'domcontentloaded' });
			await expect(page.getByText(/Request expired/i)).toBeVisible({ timeout: 15_000 });
			expect(page.url()).toContain('/authorize');
		} finally {
			await context.close();
		}
	});

	// The highest-value test here: an anonymous user arriving from a client
	// has to survive the login detour and land back on consent, not on the
	// dashboard. Nothing else covers the returnTo round trip.
	test('an anonymous visitor logs in and comes back to consent', async ({ browser, api }) => {
		const user = await createUserAccount(api);
		// A context with no cookies — the user exists, but this browser is
		// not signed in as them.
		const context = await newContext(browser);
		const page = await context.newPage();

		try {
			await stubRedirectTarget(page);
			const { requestId } = await startAuthorization();

			await page.goto(`/authorize?requestId=${requestId}`, { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/login\?returnTo=/);

			await fillLoginForm(page, { email: user.email, password: user.password });
			await submitLogin(page);

			await expectUrl(page, new RegExp(`/authorize\\?requestId=${requestId}`));
			await expect(page.getByText(/E2E Test App wants to access your Patr account/i)).toBeVisible({
				timeout: 15_000,
			});
		} finally {
			await context.close();
		}
	});

	// `returnTo` is user-controlled and then navigated to, which is an open
	// redirect unless it is pinned to this origin. Without the guard, a
	// genuine Patr login link would be able to drop the user on a convincing
	// fake immediately after they authenticate.
	test('returnTo cannot send the user off-origin after login', async ({ browser, api }) => {
		const user = await createUserAccount(api);
		const context = await newContext(browser);
		const page = await context.newPage();

		try {
			await openLoginPage(page);
			await page.goto('/login?returnTo=//evil.example/phish', {
				waitUntil: 'domcontentloaded',
			});

			await fillLoginForm(page, { email: user.email, password: user.password });
			await submitLogin(page);

			await expect(page).not.toHaveURL(/evil\.example/, { timeout: 15_000 });
			await expectUrl(page, /localhost:\d+\/$/);
		} finally {
			await context.close();
		}
	});
});
