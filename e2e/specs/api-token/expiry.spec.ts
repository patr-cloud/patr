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

  test('rejects POST that mints a token with NBF > EXP', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      createApiTokenAPI(api, user, {
        permissions: { [user.workspaceId]: { type: 'superAdmin' } },
        tokenNbf: new Date(Date.now() + 7 * 24 * 60 * 60_000),
        tokenExp: new Date(Date.now() + 24 * 60 * 60_000),
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects PATCH that lands the token in NBF > EXP', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    // Existing token has EXP set, NBF unset.
    const exp = new Date(Date.now() + 24 * 60 * 60_000);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
      tokenExp: exp,
    });
    // Patch a NBF that's strictly later than the EXP — handler should reject.
    const resp = await api
      .request('PATCH', `/user/api-token/${t.id}`, {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { tokenNbf: new Date(Date.now() + 7 * 24 * 60 * 60_000) },
      })
      .catch((err) => ({ err: String(err) }));
    expect(resp).toMatchObject({ err: expect.stringMatching(/400/) });
  });

  test('PATCH with tokenNbf: null clears the NBF', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
      tokenNbf: new Date(Date.now() - 60_000),
    });

    // get_api_token_info uses `#[serde(flatten)]` on the WithId<UserApiToken>
    // field, so token fields appear at the top level of the response.
    type TokenInfo = { tokenNbf: string | null; tokenExp: string | null };
    const before = await api.request<TokenInfo>('GET', `/user/api-token/${t.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    expect(before.tokenNbf).not.toBeNull();

    await api.request('PATCH', `/user/api-token/${t.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
      body: { tokenNbf: null },
    });

    // Response uses `#[serde(skip_serializing_if = "Option::is_none")]`, so a
    // cleared NBF surfaces as the key being absent (undefined in JS).
    const after = await api.request<TokenInfo>('GET', `/user/api-token/${t.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    expect(after.tokenNbf ?? null).toBeNull();
  });
});

test.describe('api token > allowed_ips boundary', () => {
  test('mints successfully with allowedIps: [] (normalised to no whitelist)', async ({
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const resp = await api.request<{ id: string; token: string }>(
      'POST',
      '/user/api-token',
      {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: {
          name: `tkn-empty-ips-${Date.now().toString(36)}`,
          permissions: { [user.workspaceId]: { type: 'superAdmin' } },
          allowedIps: [],
        },
      },
    );
    expect(resp.token).toMatch(/^patrv1\./);
    // And the resulting token is callable — confirming the empty list did
    // NOT get persisted as a "block all IPs" whitelist.
    const probe = await callWithApiToken(api, resp.token, {
      clientIp: user.clientIp,
      path: `/workspace/${user.workspaceId}/deployment`,
    });
    expect(probe.status).toBe(200);
  });
});
