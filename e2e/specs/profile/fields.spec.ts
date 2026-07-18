import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { openProfile } from '@/helpers/ui/profile';

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
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('profile > fields & connected accounts', () => {
	test('disables the recovery-email input', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withProfile(browser, user, async (page) => {
			await expect(page.locator('#recovery-email')).toBeDisabled({
				timeout: 10_000,
			});
		});
	});

	test('pre-fills the recovery-email input with the signup email', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withProfile(browser, user, async (page) => {
			await expect(page.locator('#recovery-email')).toHaveValue(user.email, {
				timeout: 10_000,
			});
		});
	});

	test('shows the empty connected-accounts state with a Connect GitHub button', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withProfile(browser, user, async (page) => {
			await expect(page.getByText(/No third-party accounts connected\./i)).toBeVisible({
				timeout: 10_000,
			});
			await expect(page.getByRole('button', { name: /Connect GitHub/i })).toBeVisible();
		});
	});

	test('resolves the connected-accounts loading spinner to the empty state', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withProfile(browser, user, async (page) => {
			// We accept either: "Loading..." briefly, or empty state directly. End
			// state must be the empty-state text within timeout.
			await expect(page.getByText(/No third-party accounts connected\./i)).toBeVisible({
				timeout: 15_000,
			});
		});
	});
});

test.describe('profile > inline validation', () => {
	test('rejects HTML in firstName with inline error; submit gates the PATCH', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withProfile(browser, user, async (page) => {
			let patched = false;
			page.on('request', (req) => {
				if (req.url().endsWith('/api/user') && req.method() === 'PATCH') {
					patched = true;
				}
			});
			await page.locator('#first-name').fill('<script>x</script>');
			await page.getByRole('button', { name: /^Update$/ }).click();
			await expect(
				page.getByText(/Names cannot contain <, >, &, or control characters/),
			).toBeVisible({ timeout: 5_000 });
			await page.waitForTimeout(500);
			expect(patched).toBe(false);
		});
	});

	test('rejects bracket char in lastName with inline error', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withProfile(browser, user, async (page) => {
			await page.locator('#last-name').fill('Doe<');
			await page.getByRole('button', { name: /^Update$/ }).click();
			await expect(
				page.getByText(/Names cannot contain <, >, &, or control characters/),
			).toBeVisible({ timeout: 5_000 });
		});
	});
});
