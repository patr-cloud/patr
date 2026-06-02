import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  createApiTokenAPI,
  callWithApiToken,
  loginAs,
  USER_AGENT,
} from '@/prelude';
import { openTokenList } from '@/helpers/ui/api-token';

test.describe('api token > security', () => {
  test("refuses to let one user delete another user's token", async ({ api }) => {
    await using userA = await createUserWithWorkspace(api);
    await using userB = await createUserWithWorkspace(api);
    const aToken = await createApiTokenAPI(api, userA, {
      permissions: { [userA.workspaceId]: { type: 'superAdmin' } },
    });
    // user B DELETEs A's token id.
    const r = await fetch(`${api.baseUrl}/user/api-token/${aToken.id}`, {
      method: 'DELETE',
      headers: {
        Authorization: `Bearer ${userB.accessToken}`,
        'X-Real-IP': userB.clientIp,
        'User-Agent': USER_AGENT,
      },
    });
    // After fix: expect 404. Today: returns 202 and A's token now fails.
    expect(r.status).toBe(404);
    const check = await callWithApiToken(api, aToken.token, {
      clientIp: userA.clientIp,
    });
    expect(check.status).toBe(200); // A's token should still work after fix
  });

  test("refuses to let one user regenerate another user's token", async ({ api }) => {
    await using userA = await createUserWithWorkspace(api);
    await using userB = await createUserWithWorkspace(api);
    const aToken = await createApiTokenAPI(api, userA, {
      permissions: { [userA.workspaceId]: { type: 'superAdmin' } },
    });
    const r = await fetch(`${api.baseUrl}/user/api-token/${aToken.id}/regenerate`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${userB.accessToken}`,
        'X-Real-IP': userB.clientIp,
        'User-Agent': USER_AGENT,
      },
    });
    // After fix: expect 404. Today: returns 202.
    expect(r.status).toBe(404);
    const check = await callWithApiToken(api, aToken.token, {
      clientIp: userA.clientIp,
    });
    expect(check.status).toBe(200);
  });

  test("denies cross-user data access via another user's token", async ({ api }) => {
    await using userA = await createUserWithWorkspace(api);
    await using userB = await createUserWithWorkspace(api);
    const aToken = await createApiTokenAPI(api, userA, {
      permissions: { [userA.workspaceId]: { type: 'superAdmin' } },
    });
    // A's token attempts to read B's workspace info → must be denied.
    const r = await callWithApiToken(api, aToken.token, {
      clientIp: userA.clientIp,
      path: `/workspace/${userB.workspaceId}`,
    });
    expect(r.status).toBeGreaterThanOrEqual(400);
  });

  test('returns a client-error (no 5xx) for a garbage Authorization header', async ({ api }) => {
    const res = await fetch(`${api.baseUrl}/user/api-token`, {
      method: 'POST',
      headers: {
        Authorization: 'Bearer not-a-token',
        'Content-Type': 'application/json',
        'User-Agent': USER_AGENT,
      },
      body: JSON.stringify({ name: 'x', permissions: {} }),
    });
    expect(res.status).toBeLessThan(500);
  });

  test('renders a suspicious token name as plain text (no script execution)', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    // RESOURCE_NAME_REGEX disallows <>;'`/ etc. So a true XSS payload won't
    // pass server validation. Use a still-suspicious name that DOES pass:
    // letters/digits/_-. spaces. We assert the name renders as plain text and
    // no dialog fires.
    const sneaky = `script_alert_1_${Date.now().toString(36)}`;
    const t = await createApiTokenAPI(api, user, {
      name: sneaky,
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    let dialogFired = false;
    page.on('dialog', () => {
      dialogFired = true;
    });
    try {
      await openTokenList(page);
      await expect(page.getByText(t.name)).toBeVisible({ timeout: 10_000 });
      expect(dialogFired).toBe(false);
    } finally {
      await context.close();
    }
  });

  test('rejects a PATCH targeting another user\'s token with a permissions body (IDOR)', async ({
    api,
  }) => {
    await using userA = await createUserWithWorkspace(api);
    await using userB = await createUserAccount(api);

    // A mints a token.
    const tA = await createApiTokenAPI(api, userA, {
      permissions: { [userA.workspaceId]: { type: 'superAdmin' } },
    });

    // B tries to PATCH A's token. Without the rows_affected guard the DELETE
    // block below the UPDATE would wipe A's permission rows even though the
    // UPDATE itself no-ops (token_id doesn't belong to B's user_id).
    const resp = await api
      .request('PATCH', `/user/api-token/${tA.id}`, {
        token: userB.accessToken,
        clientIp: userB.clientIp,
        body: {
          permissions: { [userA.workspaceId]: { type: 'superAdmin' } },
        },
      })
      .catch((err) => ({ err: String(err) }));
    expect(resp).toMatchObject({ err: expect.stringMatching(/404/) });

    // A's token still works against the workspace.
    const probe = await callWithApiToken(api, tA.token, {
      clientIp: userA.clientIp,
      path: `/workspace/${userA.workspaceId}/deployment`,
    });
    expect(probe.status).toBe(200);
  });

  test('API tokens cannot activate or deactivate MFA on their owning user', async ({
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });

    // activate, deactivate, and get-secret are all api = false — the API-token
    // router doesn't mount these routes at all, so the response is a 4xx
    // (exact code is Axum's "missing route" shape, not load-bearing here).
    for (const method of ['GET', 'POST', 'DELETE'] as const) {
      const res = await fetch('http://localhost:3000/user/mfa', {
        method,
        headers: {
          Authorization: t.token,
          'User-Agent': USER_AGENT,
          'X-Forwarded-For': user.clientIp,
          ...(method === 'GET' ? {} : { 'Content-Type': 'application/json' }),
        },
        body: method === 'GET' ? undefined : JSON.stringify({ otp: '000000' }),
      });
      expect(res.status).toBeGreaterThanOrEqual(400);
      expect(res.status).toBeLessThan(500);
    }
  });
});
