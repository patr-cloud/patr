import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { createContainerRepo, randomRepoName } from '@/helpers/registry';
import {
	openRegistryDetail,
	pushInstructionsHeading,
	pushInstructionsToggle,
	openPushInstructions,
	imagesEmptyState,
	deleteRepoViaModal,
	deleteTrigger,
} from '@/helpers/ui/container-registry';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';

// get-info shape, anti-enum 401, and delete-already-deleted at the API layer
// live in the Rust API suite (api/tests/api/workspace/container_registry.rs).
// Here we cover only the dashboard surface.

async function withDetail(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	id: string,
	tab: string | undefined,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openRegistryDetail(page, id, tab);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('container registry > detail [UI]', () => {
	test('Overview tab shows push instructions with the docker login command', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRepoName();
		const repo = await createContainerRepo(api, user, user.workspaceId, name);
		await withDetail(browser, user, repo.id, undefined, async (page) => {
			// Fresh repo size renders as "0 B".
			await expect(page.locator('input[name="repository-size"]')).toHaveValue(/0\s*B/);
			// Push instructions live behind a collapsible; expand it first. The login
			// command targets the registry host only (not the per-repo path).
			await openPushInstructions(page);
			await expect(pushInstructionsHeading(page)).toBeVisible();
			await expect(
				page.getByText('docker login registry.patr.cloud -u patr').first(),
			).toBeVisible();
		});
	});

	test('?tab=images shows the empty "No images yet" state', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		await withDetail(browser, user, repo.id, 'images', async (page) => {
			await expect(imagesEmptyState(page)).toBeVisible();
		});
	});

	test('?tab=garbage falls back to the Overview tab', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		await withDetail(browser, user, repo.id, 'garbage', async (page) => {
			// Overview shows the push-instructions collapsible; the Images tab's empty
			// state must not be present.
			await expect(pushInstructionsToggle(page)).toBeVisible();
			await expect(imagesEmptyState(page)).toBeHidden();
		});
	});

	test('delete via modal: success toast, redirect to list, row gone', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRepoName();
		const repo = await createContainerRepo(api, user, user.workspaceId, name);
		await withDetail(browser, user, repo.id, undefined, async (page) => {
			await deleteRepoViaModal(page, name);
			await expectToast(page, /Repository deleted successfully/i);
			await expectUrl(page, /\/container-registry$/, { timeout: 10_000 });
			await expect(page.getByText(name, { exact: true })).toBeHidden();
		});
	});

	test('delete confirm stays disabled until the name matches exactly', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRepoName();
		const repo = await createContainerRepo(api, user, user.workspaceId, name);
		await withDetail(browser, user, repo.id, undefined, async (page) => {
			await deleteTrigger(page).first().click();
			await page.getByText('Do You Really Want to Delete This Repository?').waitFor();
			const confirm = page.locator('button[type="submit"]', { hasText: /^Delete/ });
			await expect(confirm).toBeDisabled();
			await page.locator('input[type="text"]').last().fill('wrong-name');
			await expect(confirm).toBeDisabled();
			await page.locator('input[type="text"]').last().fill(name);
			await expect(confirm).toBeEnabled();
		});
	});
});
