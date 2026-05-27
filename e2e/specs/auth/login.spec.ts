import { test, expect, newContext, createUserAccount } from '@/prelude';

test('user can sign up via API and log in through the UI', async ({ api, browser }) => {
  await using user = await createUserAccount(api);

  const context = await newContext(browser, user.clientIp);
  const page = await context.newPage();

  try {
    await page.goto('/login');
    await page.locator('#userId').fill(user.username);
    await page.locator('#password').fill(user.password);

    // Submit button is disabled until Cloudflare Turnstile resolves a token
    // (frontend/src/routes/_logged-out/login.tsx: `disabled={!turnstileToken()}`).
    // Waiting for it to be enabled is the user-facing signal that Turnstile
    // is ready.
    const submitButton = page.locator('button[type=submit]', { hasText: /^Login$/ });
    await expect(submitButton).toBeEnabled({ timeout: 15_000 });

    // Submit and wait for the API to confirm the credentials.
    const signInResponse = page.waitForResponse(
      (r) => r.url().includes('/auth/sign-in') && r.request().method() === 'POST',
      { timeout: 15_000 },
    );
    await submitButton.click();
    const response = await signInResponse;
    expect(response.ok()).toBe(true);

    // The SPA persists auth state to a cookie (see `createPersistedSignal` in
    // frontend/src/hooks/state-hooks.tsx — storage is cookieStorage). The
    // route guard reads from RouterProvider context, which is reactive on the
    // auth signal. Wait for the cookie before checking URL — otherwise we
    // race the SPA's setAuthState + navigate('/').
    await page.waitForFunction(
      () => document.cookie.includes('authState='),
      null,
      { timeout: 10_000 },
    );

    // After a fresh signup the user has no workspace, so the workspaced
    // dashboard layout redirects to /onboard. Either way, we're off /login.
    await expect(page).not.toHaveURL(/\/login/, { timeout: 10_000 });
  } finally {
    await context.close();
  }
});
