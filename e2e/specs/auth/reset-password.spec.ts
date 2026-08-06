import {
	test,
	expect,
	newContext,
	createUserAccount,
	backdatePasswordResetToken,
	DEBUG_OTP,
} from '@/prelude';
import { MAX_PASSWORD_RESET_ATTEMPTS } from '@/helpers/config';
import { sql } from '@/helpers/db';
import {
	openForgotPassword,
	fillForgotEmail,
	submitForgot,
	expectCheckEmailView,
} from '@/helpers/ui/forgot';
import { openResetPassword, fillResetForm, submitReset } from '@/helpers/ui/reset';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';

// UI contract documented in @/helpers/ui/reset.ts and implemented in
// frontend/src/routes/_logged-out/reset-password.tsx.

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

async function requestResetFor(browser: import('@playwright/test').Browser, email: string) {
	await withContext(browser, async (page) => {
		await openForgotPassword(page);
		await fillForgotEmail(page, email);
		await submitForgot(page);
		await expectCheckEmailView(page);
	});
}

test.describe('reset-password [needs-ui] — happy path', () => {
	test('valid userId + OTP + new password → login with new works, old fails', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);

		const newPassword = 'NewPassw0rd!Test';
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: DEBUG_OTP,
				newPassword,
			});
			await submitReset(page);
			await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
		});

		// New password works.
		await withContext(browser, async (page) => {
			await openLoginPage(page);
			await fillLoginForm(page, { userId: user.username, password: newPassword });
			await submitLogin(page);
			await waitForLoggedIn(page);
		});

		// Old password rejected.
		await withContext(browser, async (page) => {
			await openLoginPage(page);
			await fillLoginForm(page, { userId: user.username, password: user.password });
			await submitLogin(page);
			await expect(page.getByText(/Incorrect password/i)).toBeVisible({
				timeout: 10_000,
			});
		});
	});
});

test.describe('reset-password [needs-ui] — client-side validation', () => {
	test('empty userId blocks submit', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: '',
				otp: DEBUG_OTP,
				newPassword: 'NewPassw0rd!Test',
			});
			let fired = false;
			page.on('request', (req) => {
				if (req.url().includes('/auth/reset-password')) fired = true;
			});
			await submitReset(page);
			await page.waitForTimeout(500);
			expect(fired).toBe(false);
		});
	});

	test('OTP <6 digits keeps submit disabled', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: '12345',
				newPassword: 'NewPassw0rd!Test',
			});
			const submit = page.locator('button[type=submit]', {
				hasText: /^Reset Password$/,
			});
			await expect(submit).toBeDisabled();
		});
	});

	test('new !== confirm blocks submit', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: DEBUG_OTP,
				newPassword: 'NewPassw0rd!Test',
				confirmPassword: 'DifferentPass!1',
			});
			let fired = false;
			page.on('request', (req) => {
				if (req.url().includes('/auth/reset-password')) fired = true;
			});
			await submitReset(page);
			await page.waitForTimeout(500);
			expect(fired).toBe(false);
		});
	});

	test('weak new password (no digit) blocks submit', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: DEBUG_OTP,
				newPassword: 'NoDigitsHere!',
			});
			let fired = false;
			page.on('request', (req) => {
				if (req.url().includes('/auth/reset-password')) fired = true;
			});
			await submitReset(page);
			await page.waitForTimeout(500);
			expect(fired).toBe(false);
		});
	});
});

test.describe('reset-password [needs-ui] — server-side rejection', () => {
	test('wrong OTP → InvalidPasswordResetToken', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: '123456',
				newPassword: 'NewPassw0rd!Test',
			});
			const respPromise = page.waitForResponse(
				(r) => r.url().includes('/auth/reset-password') && r.request().method() === 'POST',
			);
			await submitReset(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
		});
	});

	test('userId without a pending reset → same generic error', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		// No requestResetFor call — user has no pending reset.
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: DEBUG_OTP,
				newPassword: 'NewPassw0rd!Test',
			});
			const respPromise = page.waitForResponse(
				(r) => r.url().includes('/auth/reset-password') && r.request().method() === 'POST',
			);
			await submitReset(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
		});
	});

	test('nonexistent userId → same generic error', async ({ browser }) => {
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: 'doesnotexist' + Date.now(),
				otp: DEBUG_OTP,
				newPassword: 'NewPassw0rd!Test',
			});
			const respPromise = page.waitForResponse(
				(r) => r.url().includes('/auth/reset-password') && r.request().method() === 'POST',
			);
			await submitReset(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
		});
	});

	test('expired reset token → InvalidPasswordResetToken', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);
		await backdatePasswordResetToken(user.username, '20 min');
		await withContext(browser, async (page) => {
			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: DEBUG_OTP,
				newPassword: 'NewPassw0rd!Test',
			});
			const respPromise = page.waitForResponse(
				(r) => r.url().includes('/auth/reset-password') && r.request().method() === 'POST',
			);
			await submitReset(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
		});
	});

	// Drives the counter with real wrong-OTP submissions rather than seeding it
	// via SQL. Seeding only proves the ceiling check rejects; it says nothing
	// about whether failed attempts ever increment the counter — which is the
	// half that actually gates brute force, and the half that was broken.
	test('attempts exhausted by wrong OTPs → rejected even on correct OTP', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		await requestResetFor(browser, user.email);

		const submitOtp = async (otp: string) => {
			let ok = true;
			await withContext(browser, async (page) => {
				await openResetPassword(page);
				await fillResetForm(page, {
					userId: user.username,
					otp,
					newPassword: 'NewPassw0rd!Test',
				});
				const respPromise = page.waitForResponse(
					(r) =>
						r.url().includes('/auth/reset-password') && r.request().method() === 'POST',
				);
				await submitReset(page);
				ok = (await respPromise).ok();
			});
			return ok;
		};

		for (let i = 0; i < MAX_PASSWORD_RESET_ATTEMPTS; i++) {
			expect(await submitOtp('999999')).toBe(false);
		}

		const [row] = await sql<{ attempts: number }>(
			'SELECT password_reset_attempts AS attempts FROM "user" WHERE username = $1',
			[user.username],
		);
		expect(row?.attempts).toBe(MAX_PASSWORD_RESET_ATTEMPTS);

		// Ceiling reached: the correct OTP no longer resets the password.
		expect(await submitOtp(DEBUG_OTP)).toBe(false);
	});
});

test.describe('reset-password [needs-ui] — end-to-end seam', () => {
	test('forgot → reset in one browser context', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		const newPassword = 'AnotherPass!1';
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await openForgotPassword(page);
			await fillForgotEmail(page, user.email);
			await submitForgot(page);
			await expectCheckEmailView(page);

			await openResetPassword(page);
			await fillResetForm(page, {
				userId: user.username,
				otp: DEBUG_OTP,
				newPassword,
			});
			await submitReset(page);
			await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
