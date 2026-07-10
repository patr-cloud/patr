import {
  test,
  expect,
  newContext,
  createUserAccount,
  randomIPv4,
  TURNSTILE_TOKEN,
} from '@/prelude';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';

async function loginWith(
  browser: import('@playwright/test').Browser,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
  clientIp?: string,
) {
  const context = await newContext(browser, clientIp);
  const page = await context.newPage();
  try {
    await openLoginPage(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('login — happy paths', () => {
  test('username + password via UI lands off /login', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await loginWith(browser, async (page) => {
      await fillLoginForm(page, { userId: user.username, password: user.password });
      await submitLogin(page);
      await waitForLoggedIn(page);
    });
  });

  test('login with recovery email instead of username', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await loginWith(browser, async (page) => {
      await fillLoginForm(page, { userId: user.email, password: user.password });
      await submitLogin(page);
      await waitForLoggedIn(page);
    });
  });
});

test.describe('login — server-side rejection', () => {
  // Wait for the network response first, then assert the inline alert — under
  // sustained suite load the API can briefly take >10s and a DOM-timeout
  // assertion gets flaky.
  async function submitAndExpectInlineError(
    page: import('@playwright/test').Page,
    matcher: RegExp,
  ): Promise<void> {
    const respPromise = page.waitForResponse(
      (r) => r.url().includes('/auth/sign-in') && r.request().method() === 'POST',
      { timeout: 30_000 },
    );
    await submitLogin(page);
    const resp = await respPromise;
    expect(resp.ok()).toBe(false);
    await expect(page.getByText(matcher)).toBeVisible();
  }

  test('wrong password → inline "Incorrect password" alert', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await loginWith(browser, async (page) => {
      await fillLoginForm(page, {
        userId: user.username,
        password: 'WrongPassw0rd!',
      });
      await submitAndExpectInlineError(page, /Incorrect password/i);
      await expect(page).toHaveURL(/\/login$/);
    });
  });

  test('nonexistent username → "User not found" alert', async ({ browser }) => {
    await loginWith(browser, async (page) => {
      await fillLoginForm(page, {
        userId: 'doesnotexist' + Date.now(),
        password: 'E2eTest!1Password',
      });
      await submitAndExpectInlineError(page, /User not found/i);
    });
  });

  // Email-format input fails the userId regex preprocessor → server returns
  // a generic WrongParameters error. The frontend's only inline-alert branch
  // is `userNotFound | invalidEmail`; everything else hits the toast default.
  test('email-formatted nonexistent user → request rejected', async ({ browser }) => {
    await loginWith(browser, async (page) => {
      await fillLoginForm(page, {
        userId: `nobody${Date.now()}@example.com`,
        password: 'E2eTest!1Password',
      });
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/auth/sign-in') && r.request().method() === 'POST',
        { timeout: 10_000 },
      );
      await submitLogin(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('empty password blocks submit (no network request)', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await loginWith(browser, async (page) => {
      await page.locator('#userId').fill(user.username);
      // Password left empty.
      let fired = false;
      page.on('request', (req) => {
        if (req.url().includes('/auth/sign-in')) fired = true;
      });
      const submit = page.locator('button[type=submit]', { hasText: /^Login$/ });
      await expect(submit).toBeEnabled({ timeout: 15_000 });
      await submit.click();
      await page.waitForTimeout(500);
      expect(fired).toBe(false);
      await expect(page.getByText(/Password cannot be empty/i)).toBeVisible();
    });
  });

  // SQLi-in-userId and case-sensitive-username rejection are API-contract
  // behaviors covered in the Rust API suite (api/tests/api/auth.rs).
});

test.describe('login — concurrency & state @racy', () => {
  // page.reload() hangs against Vinxi dev (HMR-related). Instead, verify
  // session persistence by navigating to a guarded route in a fresh tab that
  // shares the same context (cookies persist).
  test('login then open new tab in same context → still logged in', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    const context = await newContext(browser);
    const page = await context.newPage();
    try {
      await openLoginPage(page);
      await fillLoginForm(page, { userId: user.username, password: user.password });
      await submitLogin(page);
      await waitForLoggedIn(page);

      const page2 = await context.newPage();
      await page2.goto('/profile');
      await expect(page2).not.toHaveURL(/\/login/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('two parallel contexts logging into same user both succeed', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    const doLogin = async () => {
      const context = await newContext(browser);
      const page = await context.newPage();
      try {
        await openLoginPage(page);
        await fillLoginForm(page, { userId: user.username, password: user.password });
        await submitLogin(page);
        await waitForLoggedIn(page);
        return true;
      } finally {
        await context.close();
      }
    };
    const results = await Promise.all([doLogin(), doLogin()]);
    expect(results).toEqual([true, true]);
  });
});

test.describe('login — rate limiting (per-IP wiring sanity)', () => {
  // The per-IP limiter is 20/sec (api/src/utils/layers/rate_limiter_layer.rs).
  // We reuse one IP across 25 sign-in attempts and assert at least one comes
  // back as 429 — proving the limiter is wired through. The exact threshold
  // is timing-sensitive so we don't assert "exactly the 21st".
  test('21+ rapid requests from one IP triggers 429 at least once', async ({ browser }) => {
    const ip = randomIPv4();
    const context = await newContext(browser, ip);
    const page = await context.newPage();
    try {
      await openLoginPage(page);
      // Hit /auth/sign-in directly from the browser so the route()
      // X-Real-IP override applies. Each call is independent.
      const statuses = await page.evaluate(async (cfTurnstileToken) => {
        const out: number[] = [];
        for (let i = 0; i < 25; i++) {
          // Relative URL — resolves against the dashboard origin Playwright
          // loaded the page from (baseURL in playwright.config). Keeps this
          // browser-context evaluate() free of localhost literals.
          const r = await fetch('/api/auth/sign-in', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              userId: 'doesnotexist',
              password: 'X',
              cfTurnstileToken,
            }),
          });
          out.push(r.status);
        }
        return out;
      }, TURNSTILE_TOKEN);
      expect(statuses.some((s) => s === 429)).toBe(true);
    } finally {
      await context.close();
    }
  });
});
