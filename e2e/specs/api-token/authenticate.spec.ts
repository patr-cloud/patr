import {
  test,
  expect,
  createUserWithWorkspace,
  createApiTokenAPI,
  callWithApiToken,
} from '@/prelude';

test.describe('api token > authentication', () => {
  test('authenticates a request with a raw API token as a Bearer header', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const t = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    const r = await callWithApiToken(api, t.token, { clientIp: user.clientIp });
    expect(r.status).toBe(200);
  });

  test('rejects a malformed token with malformedApiToken (400)', async ({ api }) => {
    const r = await callWithApiToken(api, 'patrv1.garbage');
    expect(r.status).toBe(400);
    expect(JSON.stringify(r.body).toLowerCase()).toContain('malformed');
  });

  test('rejects a well-formed but unknown token with 401', async ({ api }) => {
    // Backend Uuid parser requires non-hyphenated hex.
    const a = crypto.randomUUID().replace(/-/g, '');
    const b = crypto.randomUUID().replace(/-/g, '');
    const fake = `patrv1.${a}.${b}`;
    const r = await callWithApiToken(api, fake);
    expect(r.status).toBe(401);
  });
});
