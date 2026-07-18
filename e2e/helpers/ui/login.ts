import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend reference: frontend/src/routes/_logged-out/login.tsx
// Selectors: #userId, #password, #otp-0..#otp-5 (when MFA prompt shown),
//            button[type=submit] with text "Login".

export async function openLoginPage(page: Page): Promise<void> {
	await page.goto('/login');
}

export async function fillLoginForm(
	page: Page,
	fields: { userId: string; password: string },
): Promise<void> {
	await page.locator('#userId').fill(fields.userId);
	await page.locator('#password').fill(fields.password);
}

export async function fillMfaOtp(page: Page, otp: string): Promise<void> {
	for (let i = 0; i < 6; i++) {
		await page.locator(`#otp-${i}`).fill(otp[i] ?? '');
	}
}

// Waits for Turnstile to resolve (button enables) and submits.
export async function submitLogin(page: Page): Promise<void> {
	const submit = page.locator('button[type=submit]', { hasText: /^Login$/ });
	await expect(submit).toBeEnabled({ timeout: 15_000 });
	await submit.click();
}

// Waits for the authState cookie to land AND for the URL to leave /login.
// The cookie write is what unblocks the route guard.
export async function waitForLoggedIn(page: Page): Promise<void> {
	await page.waitForFunction(() => document.cookie.includes('authState='), null, {
		timeout: 10_000,
	});
	await expect(page).not.toHaveURL(/\/login/, { timeout: 10_000 });
}
