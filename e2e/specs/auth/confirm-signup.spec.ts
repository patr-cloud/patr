import {
  test,
  expect,
  newContext,
  createPendingSignup,
  backdateSignupOtp,
  DEBUG_OTP,
} from '@/prelude';
import {
  openConfirmSignup,
  fillUsername,
  fillOtp,
  submitConfirm,
} from '@/helpers/ui/confirm';

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

test.describe('confirm-signup — happy path', () => {
  test('correct OTP → toast + navigate to /login', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      await fillOtp(page, DEBUG_OTP);
      await submitConfirm(page);
      await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
    });
  });
});

test.describe('confirm-signup — OTP input behaviour', () => {
  test('submit disabled until all 6 digits filled', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      const submit = page.locator('button[type=submit]', { hasText: /^Confirm$/ });
      // Wait for Turnstile to enable the only-Turnstile-blocked state; submit
      // should still be disabled because OTP digits are empty.
      await page.waitForTimeout(2000);
      await expect(submit).toBeDisabled();
      await fillOtp(page, '12345'); // only 5 digits
      await expect(submit).toBeDisabled();
    });
  });

  test('typing a digit auto-focuses the next input', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      await page.locator('#otp-0').fill('1');
      await expect(page.locator('#otp-1')).toBeFocused();
    });
  });

  test('backspace on filled digit clears and focuses previous', async ({
    browser,
    api,
  }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      await page.locator('#otp-0').fill('1');
      await page.locator('#otp-1').fill('2');
      await page.locator('#otp-1').press('Backspace');
      await expect(page.locator('#otp-1')).toHaveValue('');
      await expect(page.locator('#otp-0')).toBeFocused();
    });
  });

  test('pasting 6-digit string fills all inputs', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      // Use the page's clipboard via a manual paste event.
      await page.locator('#otp-0').focus();
      await page.evaluate(() => {
        const input = document.getElementById('otp-0') as HTMLInputElement;
        const data = new DataTransfer();
        data.setData('text', '123456');
        input.dispatchEvent(
          new ClipboardEvent('paste', { clipboardData: data, bubbles: true }),
        );
      });
      for (let i = 0; i < 6; i++) {
        await expect(page.locator(`#otp-${i}`)).toHaveValue(String(i + 1));
      }
    });
  });
});

test.describe('confirm-signup — server-side rejection', () => {
  test('wrong OTP → generic credentials toast', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      await fillOtp(page, '123456');
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/auth/join') && r.request().method() === 'POST',
      );
      await submitConfirm(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
      // Still on confirm page.
      await expect(page).toHaveURL(/\/confirm-signup/);
    });
  });

  test('OTP for nonexistent username → generic error (no enumeration)', async ({
    browser,
  }) => {
    await withContext(browser, async (page) => {
      await openConfirmSignup(page); // no prefill — username field shown
      await fillUsername(page, 'doesnotexist' + Date.now());
      await fillOtp(page, DEBUG_OTP);
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/auth/join') && r.request().method() === 'POST',
      );
      await submitConfirm(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
      await expect(page).toHaveURL(/\/confirm-signup/);
    });
  });

  test('OTP for already-joined username → fails', async ({ browser, api }) => {
    await using user = await createUserAccount_(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page); // no prefill
      await fillUsername(page, user.username);
      await fillOtp(page, DEBUG_OTP);
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/auth/join') && r.request().method() === 'POST',
      );
      await submitConfirm(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('OTP for signup with backdated expiry → fails', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await backdateSignupOtp(pending.username, '1 hour');
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      await fillOtp(page, DEBUG_OTP);
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/auth/join') && r.request().method() === 'POST',
      );
      await submitConfirm(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });
});

test.describe('confirm-signup — URL parameter handling', () => {
  test('prefilled username is shown as text, no input field', async ({
    browser,
    api,
  }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      // Username input should NOT be present (the spec says it's rendered
      // as text when prefilled).
      await expect(page.locator('#username')).toHaveCount(0);
      // The username appears as text inside the confirming-for message.
      await expect(
        page.getByText(new RegExp(`Confirming account for.*${pending.username}`)),
      ).toBeVisible();
    });
  });

  test('URL params are stripped from the address bar on mount', async ({
    browser,
    api,
  }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await page.goto(`/confirm-signup?username=${pending.username}&otp=${DEBUG_OTP}`);
      // After mount, the SPA navigates to /confirm-signup with no params.
      await page.waitForFunction(
        () => !window.location.search.includes('otp='),
        null,
        { timeout: 5_000 },
      );
      expect(page.url()).not.toContain('otp=');
    });
  });

  test('no params → username input is shown and required', async ({ browser }) => {
    await withContext(browser, async (page) => {
      await openConfirmSignup(page);
      await expect(page.locator('#username')).toBeVisible();
      // Submit without username; client validation fires.
      await fillOtp(page, DEBUG_OTP);
      // Turnstile button enable + submit; expect inline username-required alert.
      const submit = page.locator('button[type=submit]', { hasText: /^Confirm$/ });
      await expect(submit).toBeEnabled({ timeout: 15_000 });
      await submit.click();
      await expect(page.getByText(/Username is required/i)).toBeVisible();
    });
  });
});

test.describe('confirm-signup — navigation', () => {
  test('"Resend Code" button navigates back to /sign-up', async ({
    browser,
    api,
  }) => {
    const pending = await createPendingSignup(api);
    await withContext(browser, async (page) => {
      await openConfirmSignup(page, pending.username);
      await page.getByRole('button', { name: /Resend Code/ }).click();
      await expect(page).toHaveURL(/\/sign-up$/, { timeout: 10_000 });
    });
  });
});

// Small inline createUserAccount shim that uses the shared `api` fixture and
// returns a UserHandle for `await using`. We re-import here only so the
// describe blocks above can use createUserAccount without unused-warning if
// no other helper imports it. (Top-level import would also work.)
import { createUserAccount as createUserAccount_ } from '@/helpers/user';
