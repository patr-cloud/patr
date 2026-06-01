import {
  test,
  expect,
  createUserWithWorkspace,
  createApiTokenAPI,
  callWithApiToken,
} from '@/prelude';

// FAILS-UNTIL-FIX: backend rejects every tokenNbf/tokenExp value (400
// wrongParameters / "Invalid body") regardless of format — ISO 8601, RFC
// 3339, epoch millis, date-only. Likely an `OffsetDateTime` serde shape
// mismatch in the preprocess macro. Once fixed, drop the `test.fail` wrappers.
test.describe('api token > expiry & NBF', () => {
  test('rejects a token whose tokenExp is in the past', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
      tokenExp: new Date(Date.now() - 60_000),
    });
    const r = await callWithApiToken(api, t.token, { clientIp: user.clientIp });
    expect(r.status).toBe(401);
  });

  test('rejects a token used before its NBF', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
      tokenNbf: new Date(Date.now() + 60 * 60_000),
    });
    const r = await callWithApiToken(api, t.token, { clientIp: user.clientIp });
    expect(r.status).toBe(401);
  });

  test('accepts a token whose NBF is now and EXP is far in the future', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
      tokenNbf: new Date(Date.now() - 60_000),
      tokenExp: new Date(Date.now() + 7 * 24 * 60 * 60_000),
    });
    const r = await callWithApiToken(api, t.token, { clientIp: user.clientIp });
    expect(r.status).toBe(200);
  });
});
