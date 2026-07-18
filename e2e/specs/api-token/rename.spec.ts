import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createApiTokenAPI,
	patchApiTokenAPI,
	loginAs,
} from '@/prelude';
import { openTokenDetail, openTokenList } from '@/helpers/ui/api-token';

test.describe('api token > rename', () => {
	test('enables the token-name input on the detail page', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openTokenDetail(page, t.id);
			await expect(page.locator('#token-name')).not.toBeDisabled({
				timeout: 10_000,
			});
		} finally {
			await context.close();
		}
	});

	test('persists a new token name set via API PATCH (visible in list)', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		const renamed = `renamed-${Date.now().toString(36)}`;
		// Update sends the full token object; resend the existing permissions.
		await patchApiTokenAPI(api, user, t.id, {
			name: renamed,
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openTokenList(page);
			await expect(page.getByText(renamed)).toBeVisible({ timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('keeps the token-id input disabled on the detail page (immutable)', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openTokenDetail(page, t.id);
			await expect(page.locator('#token-id')).toBeDisabled({ timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
