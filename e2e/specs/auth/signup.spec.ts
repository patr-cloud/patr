import {
  test,
  expect,
  newContext,
  createUserAccount,
  createPendingSignup,
  randomIPv4,
} from '@/prelude';
import { openSignupPage, fillSignupForm, submitSignup } from '@/helpers/ui/signup';

// Frontend sets `noValidate` on the form, so browser-level pattern/email
// validation is bypassed. Client-side validation is the JS `validateInputs`
// function: it checks `.trim()` non-empty + `validatePassword` + confirm-match.
// Username regex and email-format violations go through to the server and
// surface as a generic toast ("Error creating account: ...").

function newCreds(suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12)) {
  return {
    username: `e2euser${suffix}`,
    firstName: 'E2E',
    lastName: 'User',
    email: `e2euser${suffix}@example.com`,
    password: 'E2eTest!1Password',
  };
}

async function withSignupContext(
  browser: import('@playwright/test').Browser,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser);
  const page = await context.newPage();
  try {
    await openSignupPage(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('sign-up — happy path', () => {
  test('valid credentials → navigates to /confirm-signup, username pre-filled', async ({
    browser,
  }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, creds);
      await submitSignup(page);
      // The page strips ?username=... from the URL on mount, then renders
      // "Confirming account for <username>" instead of the username input.
      await expect(page).toHaveURL(/\/confirm-signup/, { timeout: 10_000 });
      await expect(
        page.getByText(new RegExp(`Confirming account for.*${creds.username}`)),
      ).toBeVisible({ timeout: 5_000 });
    });
  });
});

test.describe('sign-up — client-side field validation', () => {
  // The form blocks submit and shows inline alerts without firing a network
  // request. We verify both: the alert text AND that no /auth/sign-up request
  // fires while we wait briefly.

  async function expectNoSignupRequest(
    page: import('@playwright/test').Page,
    action: () => Promise<void>,
  ): Promise<void> {
    let fired = false;
    page.on('request', (req) => {
      if (req.url().includes('/auth/sign-up')) fired = true;
    });
    await action();
    await page.waitForTimeout(500);
    expect(fired).toBe(false);
  }

  // Note: empty `confirm-password` doesn't surface "required" — it surfaces
  // "Passwords do not match" (validateInputs only checks `!password()` for
  // required, then compares `password() !== confirmPassword()`).
  for (const field of ['username', 'first-name', 'last-name', 'email', 'password'] as const) {
    test(`empty ${field} blocks submit`, async ({ browser }) => {
      await withSignupContext(browser, async (page) => {
        const creds = newCreds();
        await fillSignupForm(page, creds);
        // Clear the field under test.
        await page.locator(`#${field}`).fill('');
        await expectNoSignupRequest(page, async () => {
          await submitSignup(page);
        });
        // An Alert appears for the field (each cleared field is required).
        await expect(page.getByText(/required/i).first()).toBeVisible();
      });
    });
  }

  test('empty confirm-password surfaces "do not match"', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, creds);
      await page.locator('#confirm-password').fill('');
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/do not match/i)).toBeVisible();
    });
  });

  test('confirm-password mismatch blocks submit', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, confirmPassword: creds.password + 'X' });
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/Passwords do not match/i)).toBeVisible();
    });
  });

  // frontend/src/utils/validation.ts `validatePassword` doesn't check length —
  // it only validates the four char classes. A 7-char password with all four
  // classes (e.g. `Ab1!xyz`) passes client validation; the server rejects it
  // (preprocessor `length(min = 8)`). Test the server path explicitly.
  test('password too short (7 chars) → server rejects', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, password: 'Ab1!xyz' });
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
        { timeout: 10_000 },
      );
      await submitSignup(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('password missing uppercase blocks submit', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, password: 'e2etest!1password' });
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/uppercase/i)).toBeVisible();
    });
  });

  test('password missing lowercase blocks submit', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, password: 'E2ETEST!1PASSWORD' });
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/lowercase/i)).toBeVisible();
    });
  });

  test('password missing digit blocks submit', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, password: 'NoDigits!Here' });
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/digit|number/i)).toBeVisible();
    });
  });

  test('password missing special char blocks submit', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, password: 'E2etest11Password' });
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/special/i)).toBeVisible();
    });
  });

  test('whitespace-only username treated as empty', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, username: '   ' });
      await expectNoSignupRequest(page, async () => {
        await submitSignup(page);
      });
      await expect(page.getByText(/required/i).first()).toBeVisible();
    });
  });
});

test.describe('sign-up — server-side rejection (bypass client validation)', () => {
  // Username regex / email-format checks happen on the server. The frontend
  // dispatches a generic toast for unknown errors; the explicit handler
  // maps usernameUnavailable/emailUnavailable to inline alerts.

  // Wait-for-response THEN assert: the API can occasionally take 10s+ to
  // respond under sustained suite load, so polling the DOM with a fixed
  // timeout makes the assertion flaky. Waiting on the network response
  // first gives us a deterministic signal that the server has answered;
  // the inline alert is rendered synchronously after the response lands.
  async function submitAndExpectInlineError(
    page: import('@playwright/test').Page,
    matcher: RegExp,
  ): Promise<void> {
    const respPromise = page.waitForResponse(
      (r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
      { timeout: 30_000 },
    );
    await submitSignup(page);
    const resp = await respPromise;
    expect(resp.ok()).toBe(false);
    await expect(page.getByText(matcher)).toBeVisible();
  }

  test('username already taken (active user) → inline alert', async ({ browser, api }) => {
    await using existing = await createUserAccount(api);
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, username: existing.username });
      await submitAndExpectInlineError(page, /Username is already taken/i);
    });
  });

  test('username already taken by pending (unconfirmed) signup', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, username: pending.username });
      await submitAndExpectInlineError(page, /Username is already taken/i);
    });
  });

  test('email already used by active user', async ({ browser, api }) => {
    await using existing = await createUserAccount(api);
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, email: existing.email });
      await submitAndExpectInlineError(page, /Email is already in use/i);
    });
  });

  test('email already used by pending signup', async ({ browser, api }) => {
    const pending = await createPendingSignup(api);
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, email: pending.email });
      await submitAndExpectInlineError(page, /Email is already in use/i);
    });
  });

  // Username regex violations get rejected by the server's WrongParameters
  // preprocessor and surface as a generic toast (Error creating account: ...).
  for (const [label, username] of [
    ['uppercase', 'BadUser123'],
    ['leading hyphen', '-baduser'],
    ['trailing hyphen', 'baduser-'],
    ['leading dot', '.baduser'],
    ['trailing dot', 'baduser.'],
    ['contains space', 'bad user'],
    ['single char', 'a'],
  ] as const) {
    test(`username regex (${label}) → server rejects`, async ({ browser }) => {
      await withSignupContext(browser, async (page) => {
        const creds = newCreds();
        await fillSignupForm(page, { ...creds, username });
        const signupResp = page.waitForResponse(
          (r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
          { timeout: 10_000 },
        );
        await submitSignup(page);
        const resp = await signupResp;
        expect(resp.ok()).toBe(false);
      });
    });
  }

  test('email missing @ → server rejects', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, { ...creds, email: 'not-an-email' });
      const signupResp = page.waitForResponse(
        (r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
        { timeout: 10_000 },
      );
      await submitSignup(page);
      const resp = await signupResp;
      expect(resp.ok()).toBe(false);
    });
  });

  test('double-submit fires only one network request', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, creds);
      let signupCalls = 0;
      page.on('request', (req) => {
        if (req.url().includes('/auth/sign-up') && req.method() === 'POST') {
          signupCalls++;
        }
      });
      const submit = page.locator('button[type=submit]', { hasText: /^Sign Up$/ });
      await expect(submit).toBeEnabled({ timeout: 15_000 });
      // Dispatch both clicks synchronously in the page. Racing two locator
      // clicks is flaky: the first click's navigation removes the button, and
      // the second click then retries actionability until it times out —
      // Playwright clicks retry rather than reject, so its .catch() never
      // fires. In-page dispatch guarantees both clicks land before any
      // navigation, which is also the truer test of the double-submit guard.
      await submit.evaluate((button: HTMLButtonElement) => {
        button.click();
        button.click();
      });
      await page.waitForURL(/\/confirm-signup/, { timeout: 10_000 });
      expect(signupCalls).toBe(1);
    });
  });
});

test.describe('sign-up — concurrency @racy', () => {
  // The API's create_account handler does an UPSERT on user_to_sign_up
  // (ON CONFLICT username DO UPDATE WHERE EXCLUDED.otp_expiry > NOW()).
  // Two concurrent signups for the same username typically both succeed
  // (the second overwrites the first), but the race can also leave one
  // rejected if the conditional UPDATE evaluates false at the moment the
  // second insert lands. Either outcome is acceptable; what's NOT
  // acceptable is "both rejected" — that would mean the row is unowned.
  test('two parallel contexts with the same username — at least one succeeds', async ({
    browser,
  }) => {
    const creds = newCreds();
    const run = async () => {
      const context = await newContext(browser, randomIPv4());
      const page = await context.newPage();
      try {
        await openSignupPage(page);
        await fillSignupForm(page, creds);
        const respPromise = page.waitForResponse(
          (r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
          { timeout: 15_000 },
        );
        await submitSignup(page);
        const resp = await respPromise;
        return resp.ok();
      } finally {
        await context.close();
      }
    };
    const results = await Promise.all([run(), run()]);
    expect(results.some(Boolean)).toBe(true);
  });
});

test.describe('sign-up — XSS-character validation', () => {
  test('rejects script-tag firstName with inline error', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, creds);
      await page.locator('#first-name').fill('<script>x</script>');
      let fired = false;
      page.on('request', (req) => {
        if (req.url().includes('/auth/sign-up')) fired = true;
      });
      await submitSignup(page);
      await expect(
        page.getByText(/Names cannot contain <, >, &, or control characters/).first(),
      ).toBeVisible({ timeout: 5_000 });
      await page.waitForTimeout(500);
      expect(fired).toBe(false);
    });
  });

  test('rejects bracket char in lastName with inline error', async ({ browser }) => {
    await withSignupContext(browser, async (page) => {
      const creds = newCreds();
      await fillSignupForm(page, creds);
      await page.locator('#last-name').fill('Doe<');
      await submitSignup(page);
      await expect(
        page.getByText(/Names cannot contain <, >, &, or control characters/).first(),
      ).toBeVisible({ timeout: 5_000 });
    });
  });
});
