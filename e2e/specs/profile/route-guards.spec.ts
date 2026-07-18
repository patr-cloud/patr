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

	test('redirects users with zero workspaces from /profile to /onboard', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user);
		const page = await context.newPage();
		try {
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/onboard/, { timeout: 10_000 });
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
});
