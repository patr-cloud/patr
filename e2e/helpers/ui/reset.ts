import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// NO FRONTEND PAGE EXISTS YET. The selectors below are the assumed contract
// the spec is written against — once the page is built, either match these
// selectors or update them here in one place.
//
// Assumed route: /reset-password
// Assumed fields:
//   #userId            — username or email
//   #otp-0..#otp-5     — six-digit verification token (OtpInput convention)
//   #new-password      — new password
//   #confirm-password  — confirm new password
// Assumed submit: button[type=submit] text "Reset Password"
// Assumed success: toast "Password reset. You can now log in", navigate to /login.

export async function openResetPassword(page: Page): Promise<void> {
	await page.goto('/reset-password');
}

export type ResetFields = {
	userId: string;
	otp: string;
	newPassword: string;
	confirmPassword?: string;
};

export async function fillResetForm(page: Page, fields: ResetFields): Promise<void> {
	await page.locator('#userId').fill(fields.userId);
	for (let i = 0; i < 6; i++) {
		await page.locator(`#otp-${i}`).fill(fields.otp[i] ?? '');
	}
	await page.locator('#new-password').fill(fields.newPassword);
	await page.locator('#confirm-password').fill(fields.confirmPassword ?? fields.newPassword);
}

export async function submitReset(page: Page): Promise<void> {
	const submit = page.locator('button[type=submit]', {
		hasText: /^Reset Password$/,
	});
	await expect(submit).toBeEnabled({ timeout: 15_000 });
	await submit.click();
}
