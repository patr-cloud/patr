import {
	test,
	expect,
	newContext,
	loginAs,
	createUserWithWorkspace,
	createSecondMemberWithRole,
	createRoleAPI,
	updateRoleAPI,
	getPermissionId,
} from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import {
	openCreateRolePage,
	fillRoleForm,
	addWorkspaceLevelPermission,
	submitCreateRole,
	expectToast,
} from '@/helpers/ui/role';
import { openInvitePage, pickBindingRole } from '@/helpers/ui/member';

// The grant ceiling: nobody can give out a permission they don't hold. The
// rule itself is covered in the Rust API suite
// (api/tests/api/workspace/rbac/permissions/**); here we cover how the
// dashboard tells a limited member about it, before and after the 403.

type Owner = UserHandle & { workspaceId: string };

async function permIds(api: ApiClient, owner: Owner, names: string[]): Promise<string[]> {
	return Promise.all(
		names.map((name) =>
			getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name),
		),
	);
}

function permissionCheckbox(page: import('@playwright/test').Page, label: string) {
	return page
		.locator('label')
		.filter({ hasText: new RegExp(`^${label}$`) })
		.first();
}

test.describe('rbac > grant ceiling [UI]', () => {
	test('the role editor disables a permission the member does not hold', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const held = await permIds(api, owner, ['viewRoles', 'modifyRoles']);
		await using member = await createSecondMemberWithRole(api, owner, held);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openCreateRolePage(page);
			await expect(
				page.getByText(/You can only add permissions you hold across the whole workspace/),
			).toBeVisible({ timeout: 10_000 });

			const locked = permissionCheckbox(page, 'Edit Workspace');
			await expect(locked.locator('input')).toBeDisabled();
			await expect(permissionCheckbox(page, 'View Roles').locator('input')).toBeEnabled();

			await locked.hover();
			await expect(page.getByRole('tooltip')).toHaveText(
				"You don't hold this permission across the whole workspace",
			);
		} finally {
			await context.close();
		}
	});

	test('the role editor leaves every permission enabled for the owner', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const context = await newContext(browser, owner.clientIp);
		await loginAs(context, owner, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openCreateRolePage(page);
			await expect(permissionCheckbox(page, 'Edit Workspace').locator('input')).toBeEnabled({
				timeout: 10_000,
			});
			await expect(
				page.getByText(/You can only add permissions you hold across the whole workspace/),
			).toBeHidden();
		} finally {
			await context.close();
		}
	});

	test('creating a role explains the refusal when a permission was lost after load', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const [viewRoles, modifyRoles, editWorkspace] = await permIds(api, owner, [
			'viewRoles',
			'modifyRoles',
			'editWorkspace',
		]);
		await using member = await createSecondMemberWithRole(api, owner, [
			viewRoles,
			modifyRoles,
			editWorkspace,
		]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openCreateRolePage(page);
			await expect(permissionCheckbox(page, 'Edit Workspace').locator('input')).toBeEnabled({
				timeout: 10_000,
			});
			// The page still believes the member holds it, so the request goes out.
			await updateRoleAPI(api, owner, owner.workspaceId, member.roleId, {
				name: `ceil-narrowed-${Date.now().toString(36)}`,
				description: 'narrowed',
				permissions: [viewRoles, modifyRoles],
			});
			await fillRoleForm(page, { name: `ceil-${Date.now().toString(36)}` });
			await addWorkspaceLevelPermission(page, 'Edit Workspace');
			await submitCreateRole(page);
			await expectToast(
				page,
				/You can only add permissions you hold across the whole workspace\. Untick/,
			);
		} finally {
			await context.close();
		}
	});

	test('the invite role picker blocks a role the member cannot grant', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const [viewRoles, modifyRoles, editWorkspace] = await permIds(api, owner, [
			'viewRoles',
			'modifyRoles',
			'editWorkspace',
		]);
		const suffix = Date.now().toString(36);
		const beyond = `ceil-beyond-${suffix}`;
		const within = `ceil-within-${suffix}`;
		await createRoleAPI(api, owner, owner.workspaceId, {
			name: beyond,
			permissions: [editWorkspace],
		});
		await createRoleAPI(api, owner, owner.workspaceId, {
			name: within,
			permissions: [viewRoles],
		});
		await using member = await createSecondMemberWithRole(api, owner, [viewRoles, modifyRoles]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openInvitePage(page);
			const addRole = page.getByRole('button', { name: /^Add role$/ });
			const explanation = page.getByText(
				/You can't grant this role here because it includes permissions you don't hold yourself/,
			);

			await pickBindingRole(page, beyond);
			await expect(explanation).toBeVisible({ timeout: 10_000 });
			await expect(addRole).toBeDisabled();

			await pickBindingRole(page, within);
			await expect(explanation).toBeHidden({ timeout: 10_000 });
			await expect(addRole).toBeEnabled();
		} finally {
			await context.close();
		}
	});
});
