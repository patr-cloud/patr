import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { createSecretAPI, randomSecretName } from '@/helpers/secret-api';
import { openSecretList, emptyStateHeading, addSecretLink, secretRow } from '@/helpers/ui/secret';
import { expectUrl } from '@/helpers/ui/workspace';

// List filtering and cross-workspace isolation at the API layer live in the
// Rust API suite (api/tests/api/workspace/secret.rs). Here we cover only the
// dashboard surface.

async function withList(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openSecretList(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('secret > list [UI]', () => {
	test('empty state shows heading and an Add Secret CTA', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withList(browser, user, async (page) => {
			await expect(emptyStateHeading(page)).toBeVisible();
			await expect(addSecretLink(page).first()).toBeVisible();
		});
	});

	test('lists secrets and shows the header Add Secret button', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		await createSecretAPI(api, user, user.workspaceId, name, 'listed-value');
		await withList(browser, user, async (page) => {
			await expect(secretRow(page, name)).toBeVisible();
			await expect(emptyStateHeading(page)).toBeHidden();
			await expect(addSecretLink(page).first()).toBeVisible();
			// The value never reaches the dashboard.
			await expect(page.getByText('listed-value')).toHaveCount(0);
		});
	});

	test('clicking a row navigates to the secret detail', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'row-value');
		await withList(browser, user, async (page) => {
			await secretRow(page, name).click();
			await expectUrl(page, new RegExp(`/secrets/${secret.id}`), { timeout: 10_000 });
		});
	});
});
