import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	loginAs,
	expectUrl,
} from '@/prelude';
import { expireAccessTokenJwt } from '@/helpers/auth';
import { openProfile, signOut } from '@/helpers/ui/profile';

test.describe('profile > route guards', () => {
	test('redirects unauthenticated visits to /profile to /login', async ({ browser }) => {
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/login/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('lets a user with zero workspaces open /profile', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user);
		const page = await context.newPage();
		try {
			// Profile is user-scoped and lives outside the _workspaced zone, so it
			// loads without a workspace — no bounce to a create/onboarding screen.
			await openProfile(page);
			await expectUrl(page, /\/profile/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('logs out from /profile and refuses to return without re-login', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openProfile(page);
			await signOut(page);
			await expectUrl(page, /\/login$/);
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/login/);
		} finally {
			await context.close();
		}
	});

	test('refreshes an expired access token and loads /profile cleanly', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		// Force the SPA's refresh path by handing it an access-token JWT whose
		// `exp` is in the past. The refresh token (`web_login.token_expiry`) is
		// still 30 days fresh, so the refresh succeeds, the retry succeeds, and
		// /profile loads — proving the auth middleware no longer rejects on the
		// session-revoke column for already-issued access tokens.
		user.accessToken = expireAccessTokenJwt(user.accessToken);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openProfile(page);
			await expect(page.locator('#first-name')).toHaveValue(user.firstName, {
				timeout: 15_000,
			});
		} finally {
			await context.close();
		}
	});

	test('a dead session (expired access + unusable refresh) is redirected to /login in SSR', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		// Expired access token AND a refresh token that can't be redeemed: the SSR
		// middleware should refresh, fail, clear the cookie, and 302 to /login —
		// before the stream flushes, so there's no logged-out flash and no
		// ERR_HTTP_HEADERS_SENT from a late Set-Cookie.
		user.accessToken = expireAccessTokenJwt(user.accessToken);
		user.refreshToken = 'deadbeef.invalidrefreshtoken';
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/login/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
