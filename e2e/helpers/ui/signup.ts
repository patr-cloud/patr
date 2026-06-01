import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend reference: frontend/src/routes/_logged-out/sign-up.tsx
// Selectors: #username, #first-name, #last-name, #email, #password,
//            #confirm-password, button[type=submit] text "Sign Up".

export type SignupFields = {
  username: string;
  firstName: string;
  lastName: string;
  email: string;
  password: string;
  confirmPassword?: string; // defaults to password
};

export async function openSignupPage(page: Page): Promise<void> {
  await page.goto('/sign-up');
}

export async function fillSignupForm(page: Page, fields: SignupFields): Promise<void> {
  await page.locator('#username').fill(fields.username);
  await page.locator('#first-name').fill(fields.firstName);
  await page.locator('#last-name').fill(fields.lastName);
  await page.locator('#email').fill(fields.email);
  await page.locator('#password').fill(fields.password);
  await page.locator('#confirm-password').fill(fields.confirmPassword ?? fields.password);
}

export async function submitSignup(page: Page): Promise<void> {
  const submit = page.locator('button[type=submit]', { hasText: /^Sign Up$/ });
  await expect(submit).toBeEnabled({ timeout: 15_000 });
  await submit.click();
}
