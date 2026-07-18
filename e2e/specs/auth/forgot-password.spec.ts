import { test, expect, newContext, createUserAccount, backdatePasswordResetToken } from '@/prelude';
import {
	openForgotPassword,
	fillForgotEmail,
	submitForgot,
	expectCheckEmailView,
} from '@/helpers/ui/forgot';

async function withContext(
	browser: import('@playwright/test').Browser,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
	const context = await newContext(browser);
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('forgot-password — happy path', () => {
	test('existing user → "Check Your Email" view', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await withContext(browser, async (page) => {
			await openForgotPassword(page);
			await fillForgotEmail(page, user.email);
			await submitForgot(page);
			await expectCheckEmailView(page);
		});
	});
});

test.describe('forgot-password — client-side validation', () => {
	test('empty email blocks submit', async ({ browser }) => {
		await withContext(browser, async (page) => {
			await openForgotPassword(page);
			let fired = false;
			page.on('request', (req) => {
				if (req.url().includes('/auth/forgot-password')) fired = true;
			});
			await submitForgot(page);
			await page.waitForTimeout(500);
			expect(fired).toBe(false);
			await expect(page.getByText(/Email.*required/i)).toBeVisible();
		});
	});
});

test.describe('forgot-password — silent successes (no enumeration)', () => {
	test('nonexistent email → same success view', async ({ browser }) => {
		await withContext(browser, async (page) => {
			await openForgotPassword(page);
			await fillForgotEmail(page, `nobody${Date.now()}@example.com`);
			await submitForgot(page);
			await expectCheckEmailView(page);
		});
	});

	test('two rapid requests for the same user → both show success view', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		await withContext(browser, async (page) => {
			await openForgotPassword(page);
			await fillForgotEmail(page, user.email);
			await submitForgot(page);
			await expectCheckEmailView(page);
			// Click "Try again" to reset the success view and fire again.
			await page.getByText(/Try again/i).click();
			await fillForgotEmail(page, user.email);
			await submitForgot(page);
			await expectCheckEmailView(page);
		});
	});
});

test.describe('forgot-password — state', () => {
	test('re-issue after backdated expiry succeeds', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await withContext(browser, async (page) => {
			await openForgotPassword(page);
			await fillForgotEmail(page, user.email);
			await submitForgot(page);
			await expectCheckEmailView(page);
		});
		// Backdate the freshly-issued token.
		await backdatePasswordResetToken(user.username, '20 min');
		// Re-issue should now succeed (the existing token is expired).
		await withContext(browser, async (page) => {
			await openForgotPassword(page);
			await fillForgotEmail(page, user.email);
			await submitForgot(page);
			await expectCheckEmailView(page);
		});
	});
});
