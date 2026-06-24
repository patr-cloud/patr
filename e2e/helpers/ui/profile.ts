import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend references:
//   - frontend/src/components/user-dropdown.tsx — top-right user menu with
//     Logout button. The dropdown opens by clicking the user button.
//   - frontend/src/routes/_logged-in/_workspaced/profile/index.tsx — profile
//     page composed of info / change-password / connected-accounts sections.
//   - .../profile/-components/change-password.tsx — selectors via
//     input[name=current-password|new-password|confirm-password], submit
//     "Update Password".
//   - .../profile/-components/info.tsx — "Enable 2FA Settings" /
//     "Disable 2FA Settings" button triggers the TwoFactorAuthModal.
//   - .../profile/-components/two-fa.tsx — modal with OtpInput (#otp-0..),
//     submit "Verify".

export async function openProfile(page: Page): Promise<void> {
  await page.goto('/profile', { waitUntil: 'domcontentloaded' });
  // Wait for SPA hydration — first-name input is the page's stable anchor.
  await page.locator('#first-name').first().waitFor({
    state: 'visible',
    timeout: 15_000,
  });
}

// Opens the user dropdown and clicks Logout. Works from any logged-in page.
export async function signOut(page: Page): Promise<void> {
  // The dropdown trigger button shows the user's display name; the safest
  // selector is the chevron-less button that contains an Initials avatar.
  // We rely on the Logout button text being unique once the dropdown opens.
  const logout = page.getByRole('button', { name: /Logout/ });
  if (!(await logout.isVisible().catch(() => false))) {
    // Open the dropdown — click the topbar button whose only role-friendly
    // identifier is the user's name; we use a more permissive locator that
    // matches any button containing the avatar + name pair. The avatar's
    // Initials component has no test id, so we fall back to selecting any
    // button in the topbar that opens a menu containing "Logout".
    const dropdownButton = page
      .locator('button')
      .filter({ has: page.locator('span.text-sm.font-medium.text-white') });
    await dropdownButton.first().click();
  }
  await page.getByRole('button', { name: /Logout/ }).click();
  await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
}

// --- Change password section ---

export async function fillChangePassword(
  page: Page,
  fields: {
    currentPassword: string;
    newPassword: string;
    confirmPassword?: string;
  },
): Promise<void> {
  await page.locator('input[name=current-password]').fill(fields.currentPassword);
  await page.locator('input[name=new-password]').fill(fields.newPassword);
  await page
    .locator('input[name=confirm-password]')
    .fill(fields.confirmPassword ?? fields.newPassword);
}

export async function fillChangePasswordMfa(page: Page, otp: string): Promise<void> {
  for (let i = 0; i < 6; i++) {
    await page.locator(`#otp-${i}`).fill(otp[i] ?? '');
  }
}

export async function submitChangePassword(page: Page): Promise<void> {
  const submit = page.getByRole('button', { name: /Update Password/ });
  await expect(submit).toBeEnabled();
  await submit.click();
}

// --- MFA section ---

export async function openMfaModal(page: Page): Promise<void> {
  // Button text alternates between "Enable 2FA Settings" and "Disable 2FA Settings".
  const button = page.getByRole('button', { name: /(Enable|Disable) 2FA Settings/ });
  await button.click();
}

export async function fillMfaModalOtp(page: Page, otp: string): Promise<void> {
  // The modal renders the same OtpInput component with #otp-0..#otp-5.
  for (let i = 0; i < 6; i++) {
    await page.locator(`#otp-${i}`).fill(otp[i] ?? '');
  }
}

export async function submitMfaModal(page: Page): Promise<void> {
  const submit = page.getByRole('button', { name: /^Verify$/ });
  await expect(submit).toBeEnabled();
  await submit.click();
}

// --- Account info (name) section ---

export async function fillNameForm(
  page: Page,
  fields: { firstName?: string; lastName?: string },
): Promise<void> {
  if (fields.firstName !== undefined) {
    await page.locator('#first-name').fill(fields.firstName);
  }
  if (fields.lastName !== undefined) {
    await page.locator('#last-name').fill(fields.lastName);
  }
}

export async function getFirstNameValue(page: Page): Promise<string> {
  return page.locator('#first-name').inputValue();
}

export async function getLastNameValue(page: Page): Promise<string> {
  return page.locator('#last-name').inputValue();
}

export async function getRecoveryEmailValue(page: Page): Promise<string> {
  return page.locator('#recovery-email').inputValue();
}

// The Update button on /profile is also used by other forms ("Update Password",
// 2FA modal). Scope to the form that contains #first-name.
export function nameUpdateButton(page: Page) {
  return page
    .locator('form')
    .filter({ has: page.locator('#first-name') })
    .getByRole('button', { name: /^Update$/ });
}

export async function submitNameUpdate(page: Page): Promise<void> {
  await nameUpdateButton(page).click();
}

export async function submitNameUpdateAndWaitResponse(
  page: Page,
): Promise<{ status: number; ok: boolean }> {
  const respPromise = page.waitForResponse(
    (r) => r.url().endsWith('/api/user') && r.request().method() === 'PATCH',
    { timeout: 30_000 },
  );
  await submitNameUpdate(page);
  const resp = await respPromise;
  return { status: resp.status(), ok: resp.ok() };
}

export async function expectUserInfoUpdateToast(
  page: Page,
  kind: 'success' | 'error',
): Promise<void> {
  const matcher =
    kind === 'success' ? /User info updated successfully/i : /Failed to update user info/i;
  await expect(page.getByText(matcher).first()).toBeVisible({ timeout: 10_000 });
}

// localInfo is hydrated by a createEffect once GET /user resolves; wait until
// the first-name input has the expected value before mutating it.
export async function reloadProfileAndWaitForUserInfo(
  page: Page,
  expectedFirstName: string,
): Promise<void> {
  await page.goto('/profile', { waitUntil: 'domcontentloaded' });
  await expect(page.locator('#first-name')).toHaveValue(expectedFirstName, {
    timeout: 10_000,
  });
}
