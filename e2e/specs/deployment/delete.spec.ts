import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { openDeploymentDetail } from '@/helpers/ui/deployment';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Covers the delete confirmation modal's UI. The modal is the shared
// DeleteModal, so this stands in for every resource's delete.

test.beforeAll(async () => {
	await seedMachineType();
});

test.describe('deployment > delete [UI]', () => {
	test('the confirmation input is focused when the modal opens', async ({ browser, api }) => {
		const user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		const dep = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id);
			const deleteButton = page.getByRole('button', { name: /^Delete$/ }).first();
			await deleteButton.waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
			await deleteButton.click();
			const modal = page.locator('form', {
				hasText: 'Do You Really Want to Delete This Deployment?',
			});
			await expect(modal.locator('input')).toBeFocused();
		} finally {
			await context.close();
		}
	});
});
