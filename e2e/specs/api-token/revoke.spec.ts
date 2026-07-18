import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createApiTokenAPI,
	callWithApiToken,
	loginAs,
} from '@/prelude';
import {
	openTokenDetail,
	clickDelete,
	fillDeleteConfirmName,
	submitDelete,
	openTokenList,
} from '@/helpers/ui/api-token';

async function withDetail(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	tokenId: string,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openTokenDetail(page, tokenId);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('api token > revoke', () => {
	test('invalidates the token after delete via UI', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		const before = await callWithApiToken(api, t.token, { clientIp: user.clientIp });
		expect(before.status).toBe(200);
		await withDetail(browser, user, t.id, async (page) => {
			await clickDelete(page);
			await fillDeleteConfirmName(page, t.name);
			await submitDelete(page);
			await expect(page.getByText(/API Token deleted successfully/i)).toBeVisible({
				timeout: 15_000,
			});
		});
		const after = await callWithApiToken(api, t.token, { clientIp: user.clientIp });
		expect(after.status).toBe(401);
	});

	test('disables the delete confirm submit until the exact name is typed', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		await withDetail(browser, user, t.id, async (page) => {
			await clickDelete(page);
			const submitBtn = page
				.locator('form')
				.filter({ hasText: /Delete API Token/i })
				.getByRole('button', { name: /^Delete$/ });
			await expect(submitBtn).toBeDisabled();
			await fillDeleteConfirmName(page, 'wrong');
			await expect(submitBtn).toBeDisabled();
			await fillDeleteConfirmName(page, t.name);
			await expect(submitBtn).toBeEnabled();
		});
	});

	test('removes a deleted token from the list', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		await api.request('DELETE', `/user/api-token/${t.id}`, {
			token: user.accessToken,
			clientIp: user.clientIp,
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openTokenList(page);
			await expect(page.getByText(t.name)).toBeHidden({ timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
