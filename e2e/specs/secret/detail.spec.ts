import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import {
	createSecretAPI,
	findSecretByName,
	getSecretAPI,
	randomSecretName,
} from '@/helpers/secret-api';
import {
	openSecretDetail,
	fillSecretName,
	fillSecretValue,
	revealSecretValue,
	updateValueButton,
	cancelValueButton,
	saveButton,
	deleteSecretViaModal,
	openSecretList,
	secretRow,
	usedByPanel,
	usedByRows,
	usedByRow,
	usedByVariable,
	usedByEmptyMessage,
	scrollUsedByToEnd,
} from '@/helpers/ui/secret';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';

// The stored value itself (OpenBao round-trips, rotation overwriting it) is
// asserted in the Rust API suite (api/tests/api/workspace/secret.rs), which
// can read OpenBao directly. Here we cover the dashboard: edits land, a
// rotation bumps `lastUpdated` while a rename doesn't, the value field stays
// hidden until asked for, and delete behaves.

test.beforeAll(async () => {
	await seedMachineType();
});

async function withDetail(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	id: string,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openSecretDetail(page, id);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('secret > detail [UI]', () => {
	test('renaming saves the new name and leaves lastUpdated alone', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName(),
			'kept-value',
		);
		const before = await getSecretAPI(api, user, user.workspaceId, secret.id);
		const newName = randomSecretName('RENAMED');
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(page.locator('#secret-name')).toHaveValue(before.name);
			await fillSecretName(page, newName);
			await saveButton(page).click();
			await expectToast(page, /Secret updated successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
		});
		const after = await getSecretAPI(api, user, user.workspaceId, secret.id);
		expect(after.name).toBe(newName);
		expect(after.lastUpdated).toBe(before.lastUpdated);
	});

	// The secret query refetches whenever the window regains focus. That must not
	// reseed the name field and wipe out a rename the user is part-way through.
	test('a refetch on window focus keeps a rename being typed', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'kept-value');
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(page.locator('#secret-name')).toHaveValue(name);
			await fillSecretName(page, 'HALF_TYPED');

			const refetch = page.waitForResponse(
				(response) =>
					response.request().method() === 'GET' &&
					response.url().includes(`/secret/${secret.id}`),
			);
			await page.evaluate(() => window.dispatchEvent(new Event('visibilitychange')));
			await refetch;

			await expect(page.locator('#secret-name')).toHaveValue('HALF_TYPED');
		});
	});

	test('rotating the value bumps lastUpdated', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'old-value');
		const before = await getSecretAPI(api, user, user.workspaceId, secret.id);
		await withDetail(browser, user, secret.id, async (page) => {
			await revealSecretValue(page);
			await fillSecretValue(page, 'new-value');
			await saveButton(page).click();
			await expectToast(page, /Secret updated successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
		});
		const after = await getSecretAPI(api, user, user.workspaceId, secret.id);
		expect(after.name).toBe(name);
		// The API suite checks the new timestamp is later; here it just has to move.
		expect(after.lastUpdated).not.toBe(before.lastUpdated);
	});

	// The value can't be read back, so there's nothing to show in a field until
	// the user asks to replace it. Cancelling puts the field away again, and a
	// save then keeps the stored value.
	test('the value field is hidden until Update value is clicked', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName(),
			'kept',
		);
		const before = await getSecretAPI(api, user, user.workspaceId, secret.id);
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(page.locator('#secret-value')).toHaveCount(0);
			await expect(
				page.getByText(/The current value is hidden and can't be viewed/),
			).toBeVisible();

			await revealSecretValue(page);
			await fillSecretValue(page, 'typed-then-cancelled');
			await cancelValueButton(page).click();
			await expect(page.locator('#secret-value')).toHaveCount(0);
			await expect(updateValueButton(page)).toBeVisible();

			await saveButton(page).click();
			await expectToast(page, /Secret updated successfully/i);
		});
		const after = await getSecretAPI(api, user, user.workspaceId, secret.id);
		expect(after.lastUpdated).toBe(before.lastUpdated);
	});

	test('delete via modal: success toast, redirect to list, secret gone', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'doomed');
		await withDetail(browser, user, secret.id, async (page) => {
			await deleteSecretViaModal(page, name);
			await expectToast(page, /Secret deleted successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
			await expect(page.getByText(name, { exact: true })).toBeHidden();
		});
		expect(await findSecretByName(api, user, user.workspaceId, name)).toBeUndefined();
	});

	test('deleting a secret a deployment uses is refused with a toast', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'in-use');
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
			environmentVariables: { API_KEY: { fromSecret: secret.id } },
		});
		await withDetail(browser, user, secret.id, async (page) => {
			await deleteSecretViaModal(page, name);
			await expectToast(page, /Secret is in use by deployment/i);
			await expectUrl(page, new RegExp(`/secrets/${secret.id}`), { timeout: 5_000 });
		});
		expect(await findSecretByName(api, user, user.workspaceId, name)).toBeDefined();
	});
});

test.describe('secret > used by [UI]', () => {
	test('lists each deployment with its variable, and links to it', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'in-use');
		const other = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName('OTHER'),
			'other',
		);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		const base = { repositoryId: repo.id, runnerId: runner.id };
		const sameName = await createDeploymentAPI(api, user, user.workspaceId, {
			...base,
			environmentVariables: { [name]: { fromSecret: secret.id } },
		});
		const otherName = await createDeploymentAPI(api, user, user.workspaceId, {
			...base,
			environmentVariables: { RENAMED_KEY: { fromSecret: secret.id } },
		});
		const unrelated = await createDeploymentAPI(api, user, user.workspaceId, {
			...base,
			environmentVariables: { RENAMED_KEY: { fromSecret: other.id } },
		});
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(usedByPanel(page).getByText('2 deployments', { exact: true })).toBeVisible(
				{
					timeout: 15_000,
				},
			);
			await expect(usedByRows(page)).toHaveCount(2);
			await expect(usedByRow(page, unrelated.name)).toHaveCount(0);

			// Only a variable named differently from the secret is highlighted.
			await expect(usedByVariable(page, sameName.name, name)).not.toHaveClass(/text-primary/);
			await expect(usedByVariable(page, otherName.name, 'RENAMED_KEY')).toHaveClass(
				/text-primary/,
			);

			await usedByRow(page, otherName.name)
				.getByRole('link', { name: otherName.name })
				.click();
			await expectUrl(page, new RegExp(`/deployments/${otherName.id}`), { timeout: 10_000 });
		});
	});

	test('a deployment using the secret twice is one row with both variables', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName(),
			'twice',
		);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		const deployment = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
			environmentVariables: {
				FIRST_KEY: { fromSecret: secret.id },
				SECOND_KEY: { fromSecret: secret.id },
			},
		});
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(usedByPanel(page).getByText('1 deployment', { exact: true })).toBeVisible({
				timeout: 15_000,
			});
			await expect(usedByRows(page)).toHaveCount(1);
			await expect(usedByVariable(page, deployment.name, 'FIRST_KEY')).toBeVisible();
			await expect(usedByVariable(page, deployment.name, 'SECOND_KEY')).toBeVisible();
		});
	});

	test('an unused secret shows the empty message beside the form', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName(),
			'unused',
		);
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(usedByEmptyMessage(page)).toBeVisible({ timeout: 15_000 });
			await expect(saveButton(page)).toBeVisible();
		});
	});

	test('scrolling the list loads the next page', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName(),
			'popular',
		);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		// The panel loads 25 at a time, so this is one more than the first page.
		for (let i = 0; i < 26; i++) {
			await createDeploymentAPI(api, user, user.workspaceId, {
				repositoryId: repo.id,
				runnerId: runner.id,
				environmentVariables: { API_KEY: { fromSecret: secret.id } },
			});
		}
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(
				usedByPanel(page).getByText('26 deployments', { exact: true }),
			).toBeVisible({
				timeout: 15_000,
			});
			await expect(async () => {
				await scrollUsedByToEnd(page);
				await expect(usedByRows(page)).toHaveCount(26, { timeout: 1_000 });
			}).toPass({ timeout: 15_000 });
		});
	});

	test('a failed list leaves the form usable, and Retry recovers', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'kept-value');
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			let failing = true;
			// fallback() keeps the context's x-real-ip route once it stops failing.
			await page.route('**/api/workspace/*/secret/*/deployment*', async (route) => {
				if (!failing) {
					await route.fallback();
					return;
				}
				await route.fulfill({
					status: 500,
					contentType: 'application/json',
					body: JSON.stringify({
						success: false,
						error: 'serverError',
						message: 'failed',
					}),
				});
			});

			// Arrive through the list so this query runs in the browser, where the
			// route above can see it, rather than during server rendering.
			await openSecretList(page);
			await secretRow(page, name).click();
			await expectUrl(page, new RegExp(`/secrets/${secret.id}`), { timeout: 10_000 });

			const error = usedByPanel(page).getByText(
				"Couldn't load the deployments using this secret.",
			);
			// Queries retry with a backoff before they surface an error.
			await expect(error).toBeVisible({ timeout: 30_000 });
			await expect(page.locator('#secret-name')).toHaveValue(name);
			await expect(saveButton(page)).toBeEnabled();

			failing = false;
			await usedByPanel(page).getByRole('button', { name: 'Retry' }).click();
			await expect(usedByEmptyMessage(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});
});
