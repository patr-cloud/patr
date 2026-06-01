import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  createApiTokenAPI,
  callWithApiToken,
  loginAs,
} from '@/prelude';
import {
  openNewTokenPage,
  openTokenDetail,
  fillTokenName,
  enableWorkspaceCheckbox,
  selectSuperAdminRadio,
  clickCreateToken,
  readNewTokenFromModal,
  setTokenNbfInput,
  setTokenExpInput,
} from '@/helpers/ui/api-token';

async function withCreate(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openNewTokenPage(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('api token > create', () => {
  test('creates a super-admin token via UI and shows the success modal', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, `ui-${Date.now().toString(36)}`);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      await clickCreateToken(page);
      await expect(page.getByText(/API Token Created Successfully/i)).toBeVisible({
        timeout: 15_000,
      });
      await expect(
        page.getByText(/Please copy your API token now\. You won't be able to see it again!/i),
      ).toBeVisible();
    });
  });

  test('authenticates with the super-admin token created via UI', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    let token = '';
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, `auth-${Date.now().toString(36)}`);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      await clickCreateToken(page);
      token = await readNewTokenFromModal(page);
    });
    const r = await callWithApiToken(api, token, { clientIp: user.clientIp });
    expect(r.status).toBe(200);
  });

  test('authenticates with a member-scoped token created via API', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    // Resolve a single permission id (deployment::view) for a member token.
    const perms = await api.request<{ permissions: { id: string; name: string }[] }>(
      'GET',
      `/workspace/${user.workspaceId}/rbac/permission`,
      { token: user.accessToken, clientIp: user.clientIp },
    );
    const viewPermId = perms.permissions.find((p) => p.name === 'deployment::view')?.id;
    expect(viewPermId).toBeTruthy();
    // For test 8 we use the API directly (UI's PermissionSelector is rich and
    // out of scope here; test 6/7 cover the super-admin UI path). The goal is
    // to prove a member token created via API also works.
    const token = await createApiTokenAPI(api, user, {
      permissions: {
        [user.workspaceId]: {
          type: 'member',
          [viewPermId!]: { permissionType: 'exclude', resources: [] },
        } as any,
      },
    });
    const r = await callWithApiToken(api, token.token, { clientIp: user.clientIp });
    expect(r.status).toBe(200);
  });

  test('denies an action that the member token does not have permission for', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const perms = await api.request<{ permissions: { id: string; name: string }[] }>(
      'GET',
      `/workspace/${user.workspaceId}/rbac/permission`,
      { token: user.accessToken, clientIp: user.clientIp },
    );
    const viewId = perms.permissions.find((p) => p.name === 'deployment::view')!.id;
    const token = await createApiTokenAPI(api, user, {
      permissions: {
        [user.workspaceId]: {
          type: 'member',
          [viewId]: { permissionType: 'exclude', resources: [] },
        } as any,
      },
    });
    // Try to call an endpoint requiring deployment::create — POST a deployment.
    const r = await callWithApiToken(api, token.token, {
      clientIp: user.clientIp,
      method: 'POST',
      path: `/workspace/${user.workspaceId}/deployment`,
    });
    expect(r.status).toBeGreaterThanOrEqual(400);
    expect(r.status).not.toBe(200);
  });

  test('disables the Create Token button until at least one workspace is enabled', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, 'never-submitted');
      await expect(page.getByRole('button', { name: /^Create Token$/ })).toBeDisabled();
    });
  });

  test('enables the Create Token button once a workspace is enabled', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, 'ok-${Date.now().toString(36)}');
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      await expect(page.getByRole('button', { name: /^Create Token$/ })).toBeEnabled();
    });
  });

  test('shows a helper message when no workspace is enabled', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      // Tick then un-tick to enter a state where button is disabled but the
      // user might force a submit via Enter. Simpler: assert the helper text
      // is visible when nothing is enabled.
      await expect(
        page.getByText(/Enable at least one workspace to create an API token\./i),
      ).toBeVisible({ timeout: 10_000 });
    });
  });

  test('renders NBF and EXP date inputs with distinct name attributes', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await expect(page.locator('input[name="token-nbf"]')).toHaveCount(1);
      await expect(page.locator('input[name="token-exp"]')).toHaveCount(1);
      await expect(page.locator('input[name="token-validity"]')).toHaveCount(0);
    });
  });

  test('persists tokenNbf when set via the first date input', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const nbfIso = new Date(Date.now() + 60_000).toISOString().split('T')[0];
    let tokenId = '';
    await withCreate(browser, user, async (page) => {
      const tokenName = `nbf-${Date.now().toString(36)}`;
      await fillTokenName(page, tokenName);
      await setTokenNbfInput(page, nbfIso);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/api-token') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await clickCreateToken(page);
      const resp = await respPromise;
      const body = (await resp.json()) as { id: string };
      tokenId = body.id;
    });
    // DB cross-check
    const { sql } = await import('@/helpers/db');
    const rows = await sql<{ token_nbf: Date | null }>(
      `SELECT token_nbf FROM user_api_token WHERE token_id = $1`,
      [tokenId],
    );
    expect(rows[0].token_nbf).not.toBeNull();
  });

  test('persists tokenExp when set via the second date input', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const expIso = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0];
    let tokenId = '';
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, `exp-${Date.now().toString(36)}`);
      await setTokenExpInput(page, expIso);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/api-token') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await clickCreateToken(page);
      const resp = await respPromise;
      const body = (await resp.json()) as { id: string };
      tokenId = body.id;
    });
    const { sql } = await import('@/helpers/db');
    const rows = await sql<{ token_exp: Date | null }>(
      `SELECT token_exp FROM user_api_token WHERE token_id = $1`,
      [tokenId],
    );
    expect(rows[0].token_exp).not.toBeNull();
  });

  test('rejects a duplicate token name with 409 and shows an error toast', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `dup-${Date.now().toString(36)}`;
    await createApiTokenAPI(api, user, {
      name,
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, name);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/api-token') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await clickCreateToken(page);
      const resp = await respPromise;
      expect(resp.status()).toBe(409);
      await expect(page.getByText(/Failed to create API token/i)).toBeVisible();
    });
  });

  test('allows a token name to be reused after the previous token is revoked', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `reuse-${Date.now().toString(36)}`;
    const t = await createApiTokenAPI(api, user, {
      name,
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    await api.request('DELETE', `/user/api-token/${t.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    // Should now succeed.
    const t2 = await createApiTokenAPI(api, user, {
      name,
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    expect(t2.id).toBeTruthy();
  });

  test('rejects a token name shorter than 4 characters with an error toast', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, 'abc');
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/api-token') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await clickCreateToken(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
      await expect(page.getByText(/Failed to create API token/i)).toBeVisible();
    });
  });

  test('copies the new token to the clipboard via the modal copy button', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const context = await newContext(browser, user.clientIp);
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openNewTokenPage(page);
      await fillTokenName(page, `clip-${Date.now().toString(36)}`);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      await clickCreateToken(page);
      const tokenText = await readNewTokenFromModal(page);
      // Click the copy button — CopyableField renders one near the value.
      // Try a generic Copy button; fall back to any visible button containing "Copy".
      const copyButton = page.getByRole('button', { name: /Copy/i }).first();
      await copyButton.click();
      const clipText = await page.evaluate(() => navigator.clipboard.readText());
      expect(clipText).toBe(tokenText);
    } finally {
      await context.close();
    }
  });

  test('shows the raw token only once and never re-renders it on detail', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    let tokenStr = '';
    let tokenId = '';
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, `once-${Date.now().toString(36)}`);
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/api-token') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await clickCreateToken(page);
      const resp = await respPromise;
      const body = (await resp.json()) as { id: string; token: string };
      tokenStr = body.token;
      tokenId = body.id;
      await openTokenDetail(page, tokenId);
      const content = await page.content();
      expect(content).not.toContain(tokenStr);
    });
  });
});
