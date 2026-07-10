import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  loginAs,
  expectUrl,
  expectUrlNot,
} from '@/prelude';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';
import {
  openOnboardPage,
  fillOnboardName,
  submitOnboard,
  onboardSubmitButton,
  expectToast,
  getLastWorkspaceIdCookie,
} from '@/helpers/ui/workspace';

const VALID = () => `wks-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

async function onboardWith(
  browser: import('@playwright/test').Browser,
  user: { accessToken: string; refreshToken: string; clientIp: string },
  fn: (
    page: import('@playwright/test').Page,
    context: import('@playwright/test').BrowserContext,
  ) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user as any);
  const page = await context.newPage();
  try {
    await openOnboardPage(page);
    await fn(page, context);
  } finally {
    await context.close();
  }
}

test.describe('workspace setup > route guards', () => {
  test('redirects unauthenticated visits to /onboard to /login', async ({ browser }) => {
    const context = await newContext(browser);
    const page = await context.newPage();
    try {
      await page.goto('/onboard', { waitUntil: 'domcontentloaded' });
      await expectUrl(page, /\/login/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('sends a user with zero workspaces to /onboard after login', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    const context = await newContext(browser, user.clientIp);
    const page = await context.newPage();
    try {
      await openLoginPage(page);
      await fillLoginForm(page, { userId: user.username, password: user.password });
      await submitLogin(page);
      await waitForLoggedIn(page);
      await expectUrl(page, /\/onboard$/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('sends a user with a workspace away from /onboard', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto('/onboard', { waitUntil: 'domcontentloaded' });
      await expectUrlNot(page, /\/onboard/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});

test.describe('workspace setup > happy path', () => {
  test('creates the first workspace and sets the lastWorkspaceId cookie', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page, context) => {
      const name = VALID();
      await fillOnboardName(page, name);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await submitOnboard(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(true);
      await expectToast(page, /Workspace created successfully/i);
      await expectUrlNot(page, /\/onboard/, { timeout: 10_000 });
      const cookieId = await getLastWorkspaceIdCookie(context);
      expect(cookieId).toBeTruthy();
    });
  });
});

test.describe('workspace setup > validation', () => {
  async function expectNoCreateRequest(
    page: import('@playwright/test').Page,
    interaction: () => Promise<void>,
  ): Promise<void> {
    let fired = false;
    page.on('request', (req) => {
      if (req.url().endsWith('/api/workspace') && req.method() === 'POST') {
        fired = true;
      }
    });
    await interaction();
    await page.waitForTimeout(500);
    expect(fired).toBe(false);
  }

  async function expectServerRejectionInline(page: import('@playwright/test').Page): Promise<void> {
    const respPromise = page.waitForResponse(
      (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
      { timeout: 30_000 },
    );
    await submitOnboard(page);
    const resp = await respPromise;
    expect(resp.ok()).toBe(false);
    await expect(
      page.getByText(/Failed to create workspace\. Please try a different name\./i),
    ).toBeVisible();
  }

  test('rejects an empty name with an inline alert and no POST', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await expectNoCreateRequest(page, async () => {
        await submitOnboard(page);
      });
      await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
    });
  });

  test('rejects a whitespace-only name with an inline alert', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, '   ');
      await expectNoCreateRequest(page, async () => {
        await submitOnboard(page);
      });
      await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
    });
  });

  test('rejects a name shorter than 4 characters', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, 'abc');
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a name longer than 255 characters', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, 'a'.repeat(256));
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a name containing disallowed characters', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, 'my!workspace');
      await expectServerRejectionInline(page);
    });
  });

  test('trims leading and trailing whitespace before submitting', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      const padded = '  validname-' + Date.now().toString(36) + '  ';
      const expected = padded.trim();
      await fillOnboardName(page, padded);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      const reqPromise = page.waitForRequest(
        (r) => r.url().endsWith('/api/workspace') && r.method() === 'POST',
        { timeout: 30_000 },
      );
      await submitOnboard(page);
      const [req, resp] = await Promise.all([reqPromise, respPromise]);
      expect(resp.ok()).toBe(true);
      const body = JSON.parse(req.postData() ?? '{}') as { name: string };
      expect(body.name).toBe(expected);
    });
  });

  test('rejects a name already taken by another workspace (CITEXT global unique)', async ({
    browser,
    api,
  }) => {
    const shared = `shared-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    await using userA = await createUserAccount(api);
    await api.request('POST', '/workspace', {
      token: userA.accessToken,
      clientIp: userA.clientIp,
      body: { name: shared },
    });
    await using userB = await createUserAccount(api);
    await onboardWith(browser, userB, async (page) => {
      await fillOnboardName(page, shared);
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a duplicate name with different casing', async ({ browser, api }) => {
    const base = `case-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    await using userA = await createUserAccount(api);
    await api.request('POST', '/workspace', {
      token: userA.accessToken,
      clientIp: userA.clientIp,
      body: { name: base.toLowerCase() },
    });
    await using userB = await createUserAccount(api);
    await onboardWith(browser, userB, async (page) => {
      await fillOnboardName(page, base.toUpperCase());
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a unicode-only name', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, '工作空间aaaa');
      await expectServerRejectionInline(page);
    });
  });

  test('rejects an injection-shaped name and keeps the page functional', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, `x'); DROP TABLE workspace;--`);
      await expectServerRejectionInline(page);
      await page.locator('#workspace-name').fill('');
      await fillOnboardName(page, 'abcd');
      await expect(page.locator('#workspace-name')).toHaveValue('abcd');
    });
  });
});

test.describe('workspace setup > concurrency & UX @racy', () => {
  test('fires exactly one POST on a rapid double-submit', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await fillOnboardName(page, VALID());
      let postCount = 0;
      // Hold the create POST so its success-navigation doesn't fire during the
      // test: the page stays on /onboard, so the suppressed second click still
      // has a button and teardown isn't racing a navigation. fallback() keeps
      // the context's x-real-ip route.
      await page.route('**/api/workspace', async (route) => {
        if (route.request().method() === 'POST') {
          postCount += 1;
          await new Promise((r) => setTimeout(r, 2000));
        }
        await route.fallback();
      });

      // First submit fires POST #1 (held) and disables the button via isLoading.
      await submitOnboard(page);
      // The second submit is suppressed by the isLoading guard; force the click
      // so Playwright doesn't auto-wait for the disabled button.
      await onboardSubmitButton(page)
        .click({ force: true })
        .catch(() => undefined);

      // Give a stray second POST a chance to surface, then assert exactly one
      // fired — the rapid double-submit was debounced. (The POST is still held,
      // so the page is on /onboard and teardown is clean.)
      await page.waitForTimeout(700);
      expect(postCount).toBe(1);
    });
  });

  test('clears the inline error on the next keystroke', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await submitOnboard(page);
      await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
      await page.locator('#workspace-name').fill('a');
      await expect(page.getByText(/Workspace name is required\./i)).toBeHidden();
    });
  });

  test('renders /onboard without the sidebar or topbar', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await onboardWith(browser, user, async (page) => {
      await expect(page.getByText('CREATE WORKSPACE', { exact: true })).toBeHidden();
      await expect(page.getByText('Select A Workspace', { exact: true })).toBeHidden();
    });
  });
});
