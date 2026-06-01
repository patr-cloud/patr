import { test, expect, newContext, createUserWithWorkspace, loginAs, sql } from '@/prelude';
import {
  openProfile,
  fillNameForm,
  submitNameUpdate,
  submitNameUpdateAndWaitResponse,
  expectUserInfoUpdateToast,
  reloadProfileAndWaitForUserInfo,
} from '@/helpers/ui/profile';

async function withProfile(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openProfile(page);
    // Wait for first/last name to populate from GET /user.
    await expect(page.locator('#first-name')).toHaveValue(user.firstName, {
      timeout: 10_000,
    });
    await fn(page);
  } finally {
    await context.close();
  }
}

async function dbNames(username: string): Promise<{ first: string; last: string }> {
  const rows = await sql<{ first_name: string; last_name: string }>(
    `SELECT first_name, last_name FROM "user" WHERE username = $1`,
    [username],
  );
  return { first: rows[0].first_name, last: rows[0].last_name };
}

test.describe('profile update > happy paths', () => {
  test('updates first name only, shows success toast, persists across reload', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const newFirst = `Ada${Math.random().toString(36).slice(2, 6)}`;
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: newFirst });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
      await expectUserInfoUpdateToast(page, 'success');
      await reloadProfileAndWaitForUserInfo(page, newFirst);
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe(newFirst);
    expect(db.last).toBe(user.lastName);
  });

  test('updates last name only, leaving first name unchanged', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const newLast = `Doe${Math.random().toString(36).slice(2, 6)}`;
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { lastName: newLast });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
      await expectUserInfoUpdateToast(page, 'success');
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe(user.firstName);
    expect(db.last).toBe(newLast);
  });

  test('updates both names in a single submit', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const newFirst = 'Joe';
    const newLast = 'Quux';
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: newFirst, lastName: newLast });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
      await expectUserInfoUpdateToast(page, 'success');
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe(newFirst);
    expect(db.last).toBe(newLast);
  });
});

test.describe('profile update > validation', () => {
  test('trims surrounding whitespace before persisting', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withProfile(browser, user, async (page) => {
      // HTML input.value strips surrounding whitespace on its own for
      // type="text" inputs, but the API side trims regardless. Drive via
      // API to exercise the trim behavior end-to-end.
      const resp = await api.request<unknown>('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: ' Ada ', lastName: user.lastName },
      });
      expect(resp).toBeDefined();
      await page.goto('/profile', { waitUntil: 'domcontentloaded' });
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe('Ada');
  });

  test('rejects an empty first name with 400', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: '', lastName: user.lastName },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects an empty last name with 400', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: user.firstName, lastName: '' },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects clearing both names with 400', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: '', lastName: '' },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects a whitespace-only first name with 400 (trimmed to empty)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: '   ', lastName: user.lastName },
      }),
    ).rejects.toThrow(/400/);
  });

  test('round-trips a unicode name', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: 'José', lastName: 'Núñez' });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe('José');
    expect(db.last).toBe('Núñez');
  });

  test('accepts emoji in a name', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: '🚀Rocket' });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe('🚀Rocket');
  });

  test('round-trips a multibyte CJK name within VARCHAR(100) char count', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: '山田' });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe('山田');
  });

  test('accepts a first name at the 100-character boundary', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = 'a'.repeat(100);
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: name });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe(name);
  });

  test('rejects a first name over the 100-character limit with 400', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: 'a'.repeat(101), lastName: user.lastName },
      }),
    ).rejects.toThrow(/400/);
    // DB row unchanged.
    const db = await dbNames(user.username);
    expect(db.first).toBe(user.firstName);
  });

  test('rejects a 5000-character last name with 400 (no partial writes)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: user.firstName, lastName: 'b'.repeat(5000) },
      }),
    ).rejects.toThrow(/400/);
    const db = await dbNames(user.username);
    expect(db.first).toBe(user.firstName);
    expect(db.last).toBe(user.lastName);
  });

  test('rejects an HTML-containing first name with 400', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: {
          firstName: '<script>window.__pwned=true</script>X',
          lastName: user.lastName,
        },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects a newline-containing first name with 400', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    await expect(
      api.request('PATCH', '/user', {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { firstName: 'Ada\nMore', lastName: user.lastName },
      }),
    ).rejects.toThrow(/400/);
  });

  test('round-trips an update through GET /user', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const newFirst = `Echo${Math.random().toString(36).slice(2, 6)}`;
    await withProfile(browser, user, async (page) => {
      await fillNameForm(page, { firstName: newFirst });
      await submitNameUpdate(page);
      await expectUserInfoUpdateToast(page, 'success');
    });
    const got = await api.request<{ firstName: string }>('GET', '/user', {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    expect(got.firstName).toBe(newFirst);
  });

  test('stores the second value on a rapid double submit', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const first1 = `Alpha${Math.random().toString(36).slice(2, 5)}`;
    const first2 = `Beta${Math.random().toString(36).slice(2, 5)}`;
    await withProfile(browser, user, async (page) => {
      // First submit
      await fillNameForm(page, { firstName: first1 });
      await submitNameUpdate(page);
      // Immediately change value and re-submit (sequentially, but back-to-back).
      await fillNameForm(page, { firstName: first2 });
      const { ok } = await submitNameUpdateAndWaitResponse(page);
      expect(ok).toBe(true);
      await expectUserInfoUpdateToast(page, 'success');
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe(first2);
  });
});
