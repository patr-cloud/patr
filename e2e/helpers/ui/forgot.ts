import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend reference: frontend/src/routes/_logged-out/forgot-password.tsx
// Field: #userId. Submit: button[type=submit] text "Send Reset Link".

export async function openForgotPassword(page: Page): Promise<void> {
	await page.goto('/forgot-password');
}

export async function fillForgotEmail(page: Page, userId: string): Promise<void> {
	await page.locator('#userId').fill(userId);
}

export async function submitForgot(page: Page): Promise<void> {
	const submit = page.locator('button[type=submit]', {
		hasText: /^Send Reset Link$/,
	});
	await expect(submit).toBeEnabled({ timeout: 15_000 });
	await submit.click();
}

// Asserts the post-submit "Check Your Email" success view is shown.
export async function expectCheckEmailView(page: Page): Promise<void> {
	await expect(page.getByText('Check Your Email')).toBeVisible({
		timeout: 10_000,
	});
}
