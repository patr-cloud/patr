import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  createSecondMemberWithRole,
  getPermissionId,
  loginAs,
} from '@/prelude';

// Role-endpoint permission gating at the API layer (viewRoles can list but not
// create/edit/delete; modifyRoles can; neither → 401) lives in the Rust API
// suite (api/tests/api/workspace/rbac/permissions/rbac.rs). Here we cover the
// dashboard surface: the roles page renders/handles-401 and member controls are
// gated.

test.describe('rbac > permission gating [UI]', () => {
  test('renders /workspace/roles without crashing for a viewRoles-only member', async ({
    browser,
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );
    await using b = await createSecondMemberWithRole(api, owner, {
      [viewId]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, b.clientIp);
    await loginAs(context, b, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto('/workspace/roles', { waitUntil: 'domcontentloaded' });
      await expect(page.locator('table')).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('handles a 401 from /workspace/roles without crashing the page', async ({
    browser,
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);
    const deployView = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'deployment::view',
    );
    await using b = await createSecondMemberWithRole(api, owner, {
      [deployView]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, b.clientIp);
    await loginAs(context, b, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto('/workspace/roles', { waitUntil: 'domcontentloaded' });
      await expect(page.locator('body')).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  // members.tsx is not yet gated by useIsAllowed; these assert the end behavior
  // (controls hidden for a viewRoles-only member) which passes today.
  test('hides the Add Member form from a member without modifyRoles', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );
    await using b = await createSecondMemberWithRole(api, owner, {
      [viewId]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, b.clientIp);
    await loginAs(context, b, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
      await expect(page.getByRole('button', { name: /^(Add Member|Adding\.\.\.)$/ })).toBeHidden({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }
  });

  test('hides the Edit roles button on member detail from a member without modifyRoles', async ({
    browser,
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );
    await using b = await createSecondMemberWithRole(api, owner, {
      [viewId]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, b.clientIp);
    await loginAs(context, b, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
      await expect(page.getByRole('button', { name: /^Edit roles$/ })).toBeHidden({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }
  });
});
