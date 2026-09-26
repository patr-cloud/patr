import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';
import {
	openSecretCreate,
	fillSecretName,
	fillSecretValue,
	submitCreateSecret,
	secretRow,
} from '@/helpers/ui/secret';
import { createSecretAPI, findSecretByName, randomSecretName } from '@/helpers/secret-api';

// The create form blocks an empty name or value client-side; every server
// rejection (bad name, duplicate) collapses into one generic alert. The 409 on
// a duplicate name and the name bounds live in the Rust API suite
// (api/tests/api/workspace/secret.rs).

async function withCreatePage(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openSecretCreate(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

function trackCreatePosts(page: import('@playwright/test').Page): () => number {
	let count = 0;
	page.on('request', (req) => {
		if (req.method() === 'POST' && /\/api\/workspace\/[^/]+\/secret$/.test(req.url())) {
			count += 1;
		}
	});
	return () => count;
}

test.describe('secret > create [UI]', () => {
	test('creates a secret: success toast, back to the list, row shown', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		await withCreatePage(browser, user, async (page) => {
			await fillSecretName(page, name);
			await fillSecretValue(page, 'created-value');
			await submitCreateSecret(page);
			await expectToast(page, /Secret created successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
			await expect(secretRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
		expect(await findSecretByName(api, user, user.workspaceId, name)).toBeDefined();
	});

	test('empty name: inline error and no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			const posts = trackCreatePosts(page);
			await fillSecretValue(page, 'some-value');
			await submitCreateSecret(page);
			await expect(page.getByText('Name is required.', { exact: true })).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('empty value: inline error and no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			const posts = trackCreatePosts(page);
			await fillSecretName(page, randomSecretName());
			await submitCreateSecret(page);
			await expect(page.getByText('Value is required.', { exact: true })).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('duplicate name surfaces a create error and stays on the form', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		await createSecretAPI(api, user, user.workspaceId, name, 'first-value');
		await withCreatePage(browser, user, async (page) => {
			await fillSecretName(page, name);
			await fillSecretValue(page, 'second-value');
			await submitCreateSecret(page);
			await expect(
				page.getByText('Failed to create secret. Please try again.', { exact: true }),
			).toBeVisible({ timeout: 10_000 });
			await expectUrl(page, /\/secrets\/new/, { timeout: 5_000 });
		});
	});
});
