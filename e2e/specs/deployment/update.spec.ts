import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { expectToast } from '@/helpers/ui/workspace';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, getDeploymentInfoAPI } from '@/helpers/deployment-api';
import {
	openDeploymentDetail,
	infoTab,
	infoImageTagInput,
	updateButton,
} from '@/helpers/ui/deployment';

// The update API contract (rename, runner, deployOnPush, scale, ports/env/probe
// replace-vs-keep, empty-PATCH, no-deploy-history) lives in the Rust API suite
// (api/tests/api/workspace/deployment/mod.rs). Here we cover the UI: editing the
// info form's Image Tag input persists the new tag.

test.beforeAll(async () => {
	await seedMachineType();
});

async function setup(api: import('@/prelude').ApiClient, opts: Record<string, unknown> = {}) {
	const user = await createUserWithWorkspace(api);
	const runner = await createRunnerAPI(api, user, user.workspaceId);
	const repo = await createContainerRepo(api, user, user.workspaceId);
	const dep = await createDeploymentAPI(api, user, user.workspaceId, {
		repositoryId: repo.id,
		runnerId: runner.id,
		...opts,
	});
	return { user, runner, repo, dep };
}

test.describe('deployment > update [UI]', () => {
	// The info form's Image Tag input is editable and the PATCH body includes
	// image_tag → editing it persists the new tag.
	test('editing the Image Tag in the info form persists the new tag', async ({
		browser,
		api,
	}) => {
		const { user, dep } = await setup(api, { imageTag: 'v1' });
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'info');
			await infoTab(page).click();
			const tagInput = infoImageTagInput(page);
			await expect(tagInput).toHaveValue('v1', { timeout: 15_000 });
			await tagInput.fill('v2');
			await updateButton(page).click();
			await expectToast(page, /Deployment updated successfully/i);
			// Tag is updated on the server.
			expect((await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id)).imageTag).toBe(
				'v2',
			);
		} finally {
			await context.close();
		}
	});
});
