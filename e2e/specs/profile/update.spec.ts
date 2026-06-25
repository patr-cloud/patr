import { test, expect, newContext, createUserWithWorkspace, loginAs, sql } from '@/prelude';
import {
  openProfile,
  fillNameForm,
  submitNameUpdate,
  submitNameUpdateAndWaitResponse,
  expectUserInfoUpdateToast,
  reloadProfileAndWaitForUserInfo,
  nameUpdateButton,
} from '@/helpers/ui/profile';

// Name validation at the API layer (empty/whitespace/over-100/5000-char/HTML/
// newline rejection, trim) lives in the Rust API suite (api/tests/api/user/mod.rs).
// Here we cover the profile name form end-to-end through the dashboard.

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

test.describe('profile update > happy paths [UI]', () => {
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

test.describe('profile update > form round-trips [UI]', () => {
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

  test('debounces a rapid double submit to a single PATCH', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const first1 = `Alpha${Math.random().toString(36).slice(2, 5)}`;
    const first2 = `Beta${Math.random().toString(36).slice(2, 5)}`;
    await withProfile(browser, user, async (page) => {
      let patchCount = 0;
      // Hold the first PATCH in flight so the disabled-while-pending window is
      // deterministic, instead of racing a fast local API. fallback() lets the
      // context's x-real-ip route still run.
      await page.route('**/api/user', async (route) => {
        if (route.request().method() === 'PATCH') {
          patchCount += 1;
          if (patchCount === 1) await new Promise((r) => setTimeout(r, 1500));
        }
        await route.fallback();
      });

      // First submit fires PATCH #1, which is held → the button disables.
      await fillNameForm(page, { firstName: first1 });
      await submitNameUpdate(page);
      await expect(nameUpdateButton(page)).toBeDisabled();

      // A second submit while #1 is pending is suppressed by the disabled button
      // (force the click so Playwright doesn't auto-wait for it to re-enable).
      await fillNameForm(page, { firstName: first2 });
      await nameUpdateButton(page)
        .click({ force: true })
        .catch(() => undefined);

      // Once PATCH #1 completes the button re-enables; only one request went out.
      await expect(nameUpdateButton(page)).toBeEnabled({ timeout: 10_000 });
      await expectUserInfoUpdateToast(page, 'success');
      expect(patchCount).toBe(1);
    });
    const db = await dbNames(user.username);
    expect(db.first).toBe(first1);
  });
});
