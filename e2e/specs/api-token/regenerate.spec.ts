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
	clickRegenerate,
	fillRegenerateConfirmName,
	submitRegenerate,
	readNewTokenFromModal,
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

test.describe('api token > regenerate', () => {
	test('invalidates the old token and accepts the new one after regenerate', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const original = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		let newToken = '';
		await withDetail(browser, user, original.id, async (page) => {
			await clickRegenerate(page);
			await fillRegenerateConfirmName(page, original.name);
			await submitRegenerate(page);
			newToken = await readNewTokenFromModal(page);
		});
		const oldR = await callWithApiToken(api, original.token, {
			clientIp: user.clientIp,
		});
		expect(oldR.status).toBe(401);
		const newR = await callWithApiToken(api, newToken, { clientIp: user.clientIp });
		expect(newR.status).toBe(200);
	});

	test('disables the regenerate confirm submit until the typed name matches exactly', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		await withDetail(browser, user, t.id, async (page) => {
			await clickRegenerate(page);
			const submit = page
				.locator('form')
				.filter({ hasText: /Regenerate API Token/i })
				.getByRole('button', { name: /^REGENERATE$/ });
			await expect(submit).toBeDisabled();
			await fillRegenerateConfirmName(page, 'wrong-name');
			await expect(submit).toBeDisabled();
			await fillRegenerateConfirmName(page, t.name);
			await expect(submit).toBeEnabled();
		});
	});

	test('treats the regenerate confirm field as case-sensitive', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const t = await createApiTokenAPI(api, user, {
			name: `Mixed-${Date.now().toString(36)}`,
			permissions: { [user.workspaceId]: { type: 'superAdmin' } },
		});
		await withDetail(browser, user, t.id, async (page) => {
			await clickRegenerate(page);
			const submit = page
				.locator('form')
				.filter({ hasText: /Regenerate API Token/i })
				.getByRole('button', { name: /^REGENERATE$/ });
			await fillRegenerateConfirmName(page, t.name.toLowerCase());
			await expect(submit).toBeDisabled();
			await fillRegenerateConfirmName(page, t.name);
			await expect(submit).toBeEnabled();
		});
	});
});
