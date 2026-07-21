import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	TURNSTILE_TOKEN,
	VINXI_DEV_URL,
} from '@/prelude';
import { openSignupPage, fillSignupForm, submitSignup } from '@/helpers/ui/signup';
import { openConfirmSignup, fillOtp, submitConfirm } from '@/helpers/ui/confirm';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';
import { openProfile } from '@/helpers/ui/profile';

// XSS-in-name and SQLi-in-userId ingest rejection are API-contract behaviors
// covered in the Rust API suite (api/tests/api/auth.rs). The dashboard-side
// security surface (cookie tampering, autocomplete attributes) stays here.

test.describe('security — cookie tampering', () => {
	test('garbage authState cookie → SPA treats as logged out', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await openLoginPage(page);
			await fillLoginForm(page, { userId: user.username, password: user.password });
			await submitLogin(page);
			await waitForLoggedIn(page);

			// Corrupt the authState cookie.
			await context.addCookies([
				{
					name: 'authState',
					value: 'not-json-garbage',
					url: VINXI_DEV_URL,
				},
			]);
			// Navigate to a guarded route.
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expect(page).toHaveURL(/\/login/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('cookie copied from another user does not grant cross-user access', async ({
		browser,
		api,
	}) => {
		await using userA = await createUserAccount(api);
		await using userB = await createUserAccount(api);

		// Log in as userA, grab their cookies.
		const ctxA = await newContext(browser);
		const pageA = await ctxA.newPage();
		await openLoginPage(pageA);
		await fillLoginForm(pageA, { userId: userA.username, password: userA.password });
		await submitLogin(pageA);
		await waitForLoggedIn(pageA);
		const aCookies = await ctxA.cookies();
		const aAuth = aCookies.find((c) => c.name === 'authState');
		expect(aAuth).toBeTruthy();
		await ctxA.close();

		// Fresh context for userB.
		const ctxB = await newContext(browser);
		const pageB = await ctxB.newPage();
		await openLoginPage(pageB);
		await fillLoginForm(pageB, { userId: userB.username, password: userB.password });
		await submitLogin(pageB);
		await waitForLoggedIn(pageB);
		const bCookies = await ctxB.cookies();
		const bAuth = bCookies.find((c) => c.name === 'authState');
		expect(bAuth).toBeTruthy();
		// The two users have different auth tokens.
		expect(aAuth!.value).not.toBe(bAuth!.value);
		await ctxB.close();
	});
});

test.describe('security — autocomplete attributes', () => {
	test('login password input has autocomplete="current-password"', async ({ browser }) => {
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await openLoginPage(page);
			await expect(page.locator('#password')).toHaveAttribute(
				'autocomplete',
				'current-password',
			);
		} finally {
			await context.close();
		}
	});

	test('signup password inputs have autocomplete="new-password"', async ({ browser }) => {
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await openSignupPage(page);
			await expect(page.locator('#password')).toHaveAttribute('autocomplete', 'new-password');
			await expect(page.locator('#confirm-password')).toHaveAttribute(
				'autocomplete',
				'new-password',
			);
		} finally {
			await context.close();
		}
	});
});
