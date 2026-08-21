import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend reference: frontend/src/routes/_logged-out/confirm-signup.tsx
// OTP inputs: #otp-0..#otp-5 (per OtpInput component).

export async function openConfirmSignup(page: Page, email?: string): Promise<void> {
	const path = email ? `/confirm-signup?email=${encodeURIComponent(email)}` : '/confirm-signup';
	await page.goto(path);
}

export async function fillEmail(page: Page, email: string): Promise<void> {
	await page.locator('#email').fill(email);
}

export async function fillOtp(page: Page, otp: string): Promise<void> {
	for (let i = 0; i < 6; i++) {
		await page.locator(`#otp-${i}`).fill(otp[i] ?? '');
	}
}

export async function submitConfirm(page: Page): Promise<void> {
	const submit = page.locator('button[type=submit]', { hasText: /^Confirm$/ });
	await expect(submit).toBeEnabled({ timeout: 15_000 });
	await submit.click();
}
