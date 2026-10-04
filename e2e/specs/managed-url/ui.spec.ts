import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import type { ApiClient } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { openDomainDetail } from '@/helpers/ui/domain';
import { expectToast } from '@/helpers/ui/workspace';
import {
	createVerifiedDomain,
	createManagedUrlAPI,
	listManagedUrlsAPI,
	proxyDeploymentBody,
	randomSubdomain,
} from '@/helpers/managed-url-api';

// Managed URLs are managed from the domain detail page. The create form's
// target picker is a nested deployment dropdown without a stable selector, so
// creation stays API + @docker; the dashboard surfaces tested here are the
// display row, the edit form and the delete two-step. (ProxyUrl/Redirect/verify
// remain API-only.)

test.beforeAll(async () => {
	await seedMachineType();
});

async function setup(api: ApiClient) {
	const user = await createUserWithWorkspace(api);
	const domain = await createVerifiedDomain(api, user, user.workspaceId);
	const runner = await createRunnerAPI(api, user, user.workspaceId);
	const repo = await createContainerRepo(api, user, user.workspaceId);
	const dep = await createDeploymentAPI(api, user, user.workspaceId, {
		repositoryId: repo.id,
		runnerId: runner.id,
		port: 80,
	});
	return { user, domain, dep };
}

test.describe('managed-url > detail [UI]', () => {
	test('a managed URL is listed on the domain detail page', async ({ browser, api }) => {
		const { user, domain, dep } = await setup(api);
		const sub = randomSubdomain();
		await createManagedUrlAPI(
			api,
			user,
			user.workspaceId,
			proxyDeploymentBody({
				domainId: domain.id,
				deploymentId: dep.id,
				port: 80,
				subDomain: sub,
			}),
		);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDomainDetail(page, domain.id);
			// The row renders the full URL link, and a fresh URL is not yet active.
			await expect(page.getByText(`${sub}.${domain.domain}`, { exact: false })).toBeVisible({
				timeout: 15_000,
			});
		} finally {
			await context.close();
		}
	});

	test('delete a managed URL via the domain detail row (two-step)', async ({ browser, api }) => {
		const { user, domain, dep } = await setup(api);
		const sub = randomSubdomain();
		await createManagedUrlAPI(
			api,
			user,
			user.workspaceId,
			proxyDeploymentBody({
				domainId: domain.id,
				deploymentId: dep.id,
				port: 80,
				subDomain: sub,
			}),
		);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDomainDetail(page, domain.id);
			const link = page.getByText(`${sub}.${domain.domain}`, { exact: false });
			await expect(link).toBeVisible({ timeout: 15_000 });
			// The row's trash icon is the red icon button; clicking it reveals the
			// Delete/Cancel confirm pair.
			await page.locator('button.text-red-500').first().click();
			// The row's confirm "Delete" (text-red-500) — distinct from the domain's
			// own header Delete button.
			await page.locator('button.text-red-500', { hasText: /^Delete$/ }).click();
			await expectToast(page, /Managed URL deleted successfully/i);
			await expect(link).toHaveCount(0, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('edit a managed URL path via the domain detail row', async ({ browser, api }) => {
		const { user, domain, dep } = await setup(api);
		const sub = randomSubdomain();
		const { id } = await createManagedUrlAPI(
			api,
			user,
			user.workspaceId,
			proxyDeploymentBody({
				domainId: domain.id,
				deploymentId: dep.id,
				port: 80,
				subDomain: sub,
			}),
		);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDomainDetail(page, domain.id);
			await expect(page.getByText(`${sub}.${domain.domain}`, { exact: false })).toBeVisible({
				timeout: 15_000,
			});
			// The row's pencil icon is the grey icon button next to the trash icon.
			await page.locator('button.text-gray-400').first().click();
			// The page also has the create form's Path input, so stay inside the
			// edit form.
			const form = page.locator('form', { hasText: 'Update Managed URL' });
			await form.getByPlaceholder('Path').fill('/edited');
			await form.getByRole('button', { name: /^Update$/ }).click();
			await expectToast(page, /Managed URL updated successfully/i);
			const { urls } = await listManagedUrlsAPI(api, user, user.workspaceId);
			expect(urls.find((url) => url.id === id)?.path).toBe('/edited');
		} finally {
			await context.close();
		}
	});
});
