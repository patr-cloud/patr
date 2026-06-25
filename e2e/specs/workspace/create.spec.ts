import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  createUserWithWorkspaces,
  loginAs,
  expectUrl,
} from '@/prelude';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';
import {
  openCreateWorkspacePage,
  fillOnboardName as fillCreateName, // same #workspace-name selector
  submitCreateWorkspace,
  expectToast,
  getLastWorkspaceIdCookie,
  openWorkspaceSwitcher,
  listSwitcherWorkspaceNames,
} from '@/helpers/ui/workspace';

const VALID = () => `wks-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

async function withCreate(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (
    page: import('@playwright/test').Page,
    context: import('@playwright/test').BrowserContext,
  ) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openCreateWorkspacePage(page);
    await fn(page, context);
  } finally {
    await context.close();
  }
}

test.describe('workspace create > route guards', () => {
  test('redirects unauthenticated visits to /workspace/new to /login', async ({ browser }) => {
    const context = await newContext(browser);
    const page = await context.newPage();
    try {
      await page.goto('/workspace/new', { waitUntil: 'domcontentloaded' });
      await expectUrl(page, /\/login/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('redirects a user with zero workspaces from /workspace/new to /onboard', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user);
    const page = await context.newPage();
    try {
      await page.goto('/workspace/new', { waitUntil: 'domcontentloaded' });
      await expectUrl(page, /\/onboard/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});

test.describe('workspace create > happy path', () => {
  test('creates a second workspace and navigates to /workspace', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillCreateName(page, VALID());
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await submitCreateWorkspace(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(true);
      await expectToast(page, /Workspace created successfully/i);
      await expectUrl(page, /\/workspace$/, { timeout: 10_000 });
    });
  });

  test('updates lastWorkspaceId cookie to the new workspace id', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page, context) => {
      const newName = VALID();
      await fillCreateName(page, newName);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await submitCreateWorkspace(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(true);
      const body = (await resp.json()) as { id: string };
      // Wait briefly for the SPA to apply the cookie update.
      await page.waitForTimeout(500);
      const cookieId = await getLastWorkspaceIdCookie(context);
      // Bug today: cookie still points at the original workspace.
      expect(cookieId).toBe(body.id);
    });
  });

  test('shows a confirming toast after auto-switching to the new workspace', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillCreateName(page, VALID());
      await submitCreateWorkspace(page);
      // We expect a "Now using" / "Switched to" toast — text TBD by frontend.
      await expectToast(page, /Now using|Switched to|active workspace/i, 5_000);
    });
  });

  test('lists the new workspace in the sidebar switcher after creation', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      const newName = VALID();
      await fillCreateName(page, newName);
      await submitCreateWorkspace(page);
      await expectToast(page, /Workspace created successfully/i);
      // Navigated to /workspace; switcher should now list both. Creating the
      // workspace invalidates the workspaces query, so the switcher refetches
      // asynchronously — poll until the new workspace appears rather than reading
      // a single (possibly pre-refetch) snapshot.
      await openWorkspaceSwitcher(page);
      await expect
        .poll(() => listSwitcherWorkspaceNames(page), { timeout: 10_000 })
        .toEqual(expect.arrayContaining([`wks-${user.username}`, newName]));
    });
  });

  test('renders the name input with "Enter Workspace Name" placeholder', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await expect(page.locator('#workspace-name')).toHaveAttribute(
        'placeholder',
        'Enter Workspace Name',
      );
    });
  });

  test('shows a "Creating Workspace..." loading label while the request is in flight', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      // Delay the POST so we can observe the loading state.
      await page.route('**/api/workspace', async (route) => {
        if (route.request().method() === 'POST') {
          await new Promise((r) => setTimeout(r, 800));
        }
        await route.continue();
      });
      await fillCreateName(page, VALID());
      await submitCreateWorkspace(page);
      await expect(page.getByRole('button', { name: /^Creating Workspace\.\.\.$/ })).toBeVisible({
        timeout: 5_000,
      });
    });
  });
});

test.describe('workspace create > validation', () => {
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
    await submitCreateWorkspace(page);
    const resp = await respPromise;
    expect(resp.ok()).toBe(false);
    await expect(
      page.getByText(/Failed to create workspace\. Please try a different name\./i),
    ).toBeVisible();
  }

  test('rejects an empty name with an inline alert and no POST', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await expectNoCreateRequest(page, async () => {
        await submitCreateWorkspace(page);
      });
      await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
    });
  });

  test("rejects a name already used by one of the user's own workspaces", async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillCreateName(page, `wks-${user.username}`); // exact match to seeded ws
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a name already taken by another user (global unique)', async ({ browser, api }) => {
    const shared = `shared-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    await using userA = await createUserAccount(api);
    await api.request('POST', '/workspace', {
      token: userA.accessToken,
      clientIp: userA.clientIp,
      body: { name: shared },
    });
    await using userB = await createUserWithWorkspace(api);
    await withCreate(browser, userB, async (page) => {
      await fillCreateName(page, shared);
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a duplicate name with different casing', async ({ browser, api }) => {
    const base = `caseX-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    await using userA = await createUserAccount(api);
    await api.request('POST', '/workspace', {
      token: userA.accessToken,
      clientIp: userA.clientIp,
      body: { name: base.toLowerCase() },
    });
    await using userB = await createUserWithWorkspace(api);
    await withCreate(browser, userB, async (page) => {
      await fillCreateName(page, base.toUpperCase());
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a name shorter than 4 characters', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillCreateName(page, 'abc');
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a name longer than 255 characters', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillCreateName(page, 'a'.repeat(256));
      await expectServerRejectionInline(page);
    });
  });

  test('rejects a name with disallowed characters', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillCreateName(page, 'bad@name');
      await expectServerRejectionInline(page);
    });
  });

  test('trims leading and trailing whitespace before submitting', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      const padded = '  trim-' + Date.now().toString(36) + '  ';
      const expected = padded.trim();
      const reqPromise = page.waitForRequest(
        (r) => r.url().endsWith('/api/workspace') && r.method() === 'POST',
        { timeout: 30_000 },
      );
      await fillCreateName(page, padded);
      await submitCreateWorkspace(page);
      const req = await reqPromise;
      const body = JSON.parse(req.postData() ?? '{}') as { name: string };
      expect(body.name).toBe(expected);
    });
  });

  test('renders the Workspace > New breadcrumb', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      // Breadcrumb labels rendered as Links + text per PageContainerHead.
      await expect(page.getByRole('link', { name: /^Workspace$/ })).toBeVisible();
      await expect(page.getByText(/^New$/).first()).toBeVisible();
    });
  });
});

test.describe('workspace create > concurrency', () => {
  test('serialises concurrent same-name creates from a single user (exactly one succeeds)', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const raceName = `race-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

    const submitFrom = async (): Promise<{ ok: boolean }> => {
      const context = await newContext(browser, user.clientIp);
      await loginAs(context, user, { workspaceId: user.workspaceId });
      const page = await context.newPage();
      try {
        await openCreateWorkspacePage(page);
        await fillCreateName(page, raceName);
        const respPromise = page.waitForResponse(
          (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
          { timeout: 30_000 },
        );
        await submitCreateWorkspace(page);
        const resp = await respPromise;
        return { ok: resp.ok() };
      } finally {
        await context.close();
      }
    };

    const [a, b] = await Promise.all([submitFrom(), submitFrom()]);
    // At least one succeeds; both succeeding would mean we accidentally created
    // two rows with the same name (which the unique index forbids).
    expect(a.ok || b.ok).toBe(true);
    expect(a.ok && b.ok).toBe(false);
  });

  test('serialises concurrent same-name creates across two different users', async ({
    browser,
    api,
  }) => {
    const raceName = `xrace-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    await using userA = await createUserWithWorkspace(api);
    await using userB = await createUserWithWorkspace(api);

    const submitFrom = async (
      u: Awaited<ReturnType<typeof createUserWithWorkspace>>,
    ): Promise<{ ok: boolean }> => {
      const context = await newContext(browser, u.clientIp);
      await loginAs(context, u, { workspaceId: u.workspaceId });
      const page = await context.newPage();
      try {
        await openCreateWorkspacePage(page);
        await fillCreateName(page, raceName);
        const respPromise = page.waitForResponse(
          (r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
          { timeout: 30_000 },
        );
        await submitCreateWorkspace(page);
        const resp = await respPromise;
        return { ok: resp.ok() };
      } finally {
        await context.close();
      }
    };

    const [a, b] = await Promise.all([submitFrom(userA), submitFrom(userB)]);
    expect(a.ok || b.ok).toBe(true);
    expect(a.ok && b.ok).toBe(false);
  });
});
