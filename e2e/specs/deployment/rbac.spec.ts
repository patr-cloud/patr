import {
	test,
	expect,
	newContext,
	loginAs,
	createUserWithWorkspace,
	createSecondMemberWithRole,
	getPermissionId,
} from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, patrDeploymentBody } from '@/helpers/deployment-api';
import {
	openDeploymentList,
	createDeploymentLink,
	deploymentRow,
	emptyStateHeading,
	openDeploymentDetail,
	noPermissionsHeading,
	envKeyInputs,
	configMountPathInput,
	volumePathInput,
	noEditAccessNote,
	infoImageNameInput,
	infoTab,
} from '@/helpers/ui/deployment';

// Deployment RBAC at the API layer (create/view/edit/delete/start/stop gating,
// Create≠View, membership-gated empty list, cross-workspace isolation) lives in
// the Rust API suite (api/tests/api/workspace/rbac/permissions/deployment.rs).
// Here we cover dashboard control-visibility + NoPermissionsPage.

type Owner = UserHandle & { workspaceId: string };

test.beforeAll(async () => {
	await seedMachineType();
});

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
	return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

async function ownerWithDeployment(api: ApiClient, opts: Record<string, unknown> = {}) {
	const owner = await createUserWithWorkspace(api);
	const runner = await createRunnerAPI(api, owner, owner.workspaceId);
	const repo = await createContainerRepo(api, owner, owner.workspaceId);
	const dep = await createDeploymentAPI(api, owner, owner.workspaceId, {
		repositoryId: repo.id,
		runnerId: runner.id,
		...opts,
	});
	return { owner, runner, repo, dep };
}

test.describe('deployment > RBAC [UI]', () => {
	test('a view-only member sees deployments but no Create Deployment CTA', async ({
		browser,
		api,
	}) => {
		const { owner, dep } = await ownerWithDeployment(api);
		const viewId = await permId(api, owner, 'deployment::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			await expect(deploymentRow(page, dep.name)).toBeVisible({ timeout: 15_000 });
			await expect(createDeploymentLink(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a create-only member hits NoPermissionsPage on a deployment detail', async ({
		browser,
		api,
	}) => {
		const owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		const repo = await createContainerRepo(api, owner, owner.workspaceId);
		const createId = await permId(api, owner, 'deployment::create');
		await using member = await createSecondMemberWithRole(api, owner, [createId]);
		// The member creates a deployment (allowed) but cannot view it.
		const created = await api.request<{ id: string }>(
			'POST',
			`/workspace/${owner.workspaceId}/deployment`,
			{
				token: member.accessToken,
				clientIp: member.clientIp,
				body: patrDeploymentBody({ repositoryId: repo.id, runnerId: runner.id }),
			},
		);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, created.id);
			await expect(noPermissionsHeading(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('a member with the create permission sees the Create Deployment CTA', async ({
		browser,
		api,
	}) => {
		const owner = await createUserWithWorkspace(api);
		const createId = await permId(api, owner, 'deployment::create');
		await using member = await createSecondMemberWithRole(api, owner, [createId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			await expect(createDeploymentLink(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('a view member reaches a deployment detail instead of NoPermissionsPage', async ({
		browser,
		api,
	}) => {
		const { owner, dep } = await ownerWithDeployment(api);
		const viewId = await permId(api, owner, 'deployment::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id);
			await expect(noPermissionsHeading(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a view-only member gets no add rows on the Configuration tab, and is told why', async ({
		browser,
		api,
	}) => {
		const { owner, dep } = await ownerWithDeployment(api, {
			environmentVariables: { LOG_LEVEL: 'info' },
		});
		const viewId = await permId(api, owner, 'deployment::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			for (const section of ['environment variables', 'config files', 'volumes']) {
				await expect(noEditAccessNote(page, section)).toBeVisible({ timeout: 15_000 });
			}
			// The saved variable stays readable; only the blank draft row is gone.
			await expect(envKeyInputs(page)).toHaveCount(1);
			await expect(envKeyInputs(page)).toHaveValue('LOG_LEVEL');
			await expect(envKeyInputs(page)).toBeDisabled();
			await expect(configMountPathInput(page)).toHaveCount(0);
			await expect(volumePathInput(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a member who can edit gets the add rows and no access note', async ({ browser, api }) => {
		const { owner, dep } = await ownerWithDeployment(api);
		const viewId = await permId(api, owner, 'deployment::view');
		const editId = await permId(api, owner, 'deployment::edit');
		await using member = await createSecondMemberWithRole(api, owner, [viewId, editId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await expect(configMountPathInput(page)).toBeVisible({ timeout: 15_000 });
			await expect(volumePathInput(page)).toBeVisible();
			await expect(envKeyInputs(page)).toBeEnabled();
			for (const section of ['environment variables', 'config files', 'volumes']) {
				await expect(noEditAccessNote(page, section)).toHaveCount(0);
			}
		} finally {
			await context.close();
		}
	});

	test('a member who can view the repository sees its name as the image', async ({
		browser,
		api,
	}) => {
		const { owner, repo, dep } = await ownerWithDeployment(api);
		const viewId = await permId(api, owner, 'deployment::view');
		const repoViewId = await permId(api, owner, 'containerRegistryRepository::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId, repoViewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'info');
			await infoTab(page).click();
			await expect(infoImageNameInput(page)).toHaveValue(repo.name, { timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('a member who cannot view the repository sees its ID as the image', async ({
		browser,
		api,
	}) => {
		const { owner, repo, dep } = await ownerWithDeployment(api);
		const viewId = await permId(api, owner, 'deployment::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'info');
			await infoTab(page).click();
			await expect(infoImageNameInput(page)).toHaveValue(repo.id, { timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('a member with no deployment permission sees the empty state', async ({
		browser,
		api,
	}) => {
		const { owner } = await ownerWithDeployment(api);
		const viewRoles = await permId(api, owner, 'viewRoles');
		await using member = await createSecondMemberWithRole(api, owner, [viewRoles]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});
});
