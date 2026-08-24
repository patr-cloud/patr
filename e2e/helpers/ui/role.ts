import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/workspace/roles/{index,new,$roleId}.tsx
//   .../roles/-components/{permission-picker,edit,users,role-header}.tsx

export async function openRolesList(page: Page): Promise<void> {
	// Fresh workspaces already have 36 seeded roles; use a high count so any
	// role we just created via API is visible on the first page.
	await page.goto('/workspace/roles?count=100', { waitUntil: 'domcontentloaded' });
	// The seeded 36 default roles always exist, so the table row header is the
	// stable hydration anchor; fall back to the empty-state heading just in case.
	await page
		.locator('text=/Role Name|No Roles Created/i')
		.first()
		.waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

export async function openCreateRolePage(page: Page): Promise<void> {
	await page.goto('/workspace/roles/new', { waitUntil: 'domcontentloaded' });
	await page.getByPlaceholder('Enter Name').first().waitFor({
		state: 'visible',
		timeout: HYDRATION_TIMEOUT,
	});
}

export async function openRoleDetail(page: Page, roleId: string): Promise<void> {
	await page.goto(`/workspace/roles/${roleId}`, { waitUntil: 'domcontentloaded' });
	// RoleHeader's tab links are the stable anchor.
	await page.getByRole('link', { name: /^Edit Permissions$/ }).waitFor({
		state: 'visible',
		timeout: HYDRATION_TIMEOUT,
	});
}

export async function fillRoleForm(
	page: Page,
	fields: { name?: string; description?: string },
): Promise<void> {
	if (fields.name !== undefined) {
		await page.getByPlaceholder('Enter Name').fill(fields.name);
	}
	if (fields.description !== undefined) {
		await page.getByPlaceholder('Enter Description (optional)').fill(fields.description);
	}
}

// PermissionPicker helpers. Column 1 lists the resource types: pills for the
// actioned types (click to drill into the Actions column), checkbox cards for
// the workspace-level types (viewRoles / modifyRoles / editWorkspace), which
// toggle directly. The Checkbox renders an sr-only <input> inside a <label>,
// so we click the label wrapper (same trick as the token workspace checkbox).

// Toggle a workspace-level permission's checkbox card in the picker.
export async function addWorkspaceLevelPermission(
	page: Page,
	resourceLabel: 'Modify Roles' | 'View Roles' | 'Edit Workspace',
): Promise<void> {
	const escaped = resourceLabel.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	await page
		.locator('label')
		.filter({ hasText: new RegExp(`^${escaped}$`) })
		.first()
		.click();
}

// Drill into an actioned resource type (pill) in the picker's first column.
export async function selectResourceTypePill(page: Page, label: string): Promise<void> {
	await page.getByRole('button', { name: label, exact: true }).click();
}

// Toggle one action's checkbox in the picker's second column. Select the
// resource type pill first.
export async function toggleActionCheckbox(page: Page, label: string): Promise<void> {
	const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	await page
		.locator('label')
		.filter({ hasText: new RegExp(`^${escaped}$`) })
		.first()
		.click();
}

export async function submitCreateRole(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Create Role|Creating\.\.\.)$/ }).click();
}

export async function submitSavePermissions(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Save Changes|Saving Changes\.\.\.)$/ }).click();
}

export async function openRoleUsersTab(page: Page): Promise<void> {
	await page.getByRole('link', { name: /^Users$/ }).click();
}

export async function clickDeleteRole(page: Page, roleName: string): Promise<void> {
	// Each row has two <button>s — the "See users" expand toggle and the trash —
	// so locate the trash by its accessible name rather than by position.
	const row = page.getByRole('row').filter({ hasText: roleName });
	await row.scrollIntoViewIfNeeded({ timeout: 10_000 }).catch(() => {});
	await row.getByRole('button', { name: /Delete role/i }).click();
}

export async function confirmDeleteRoleModal(page: Page): Promise<void> {
	// DeleteModal asks the user to type the resource name. Find the form and
	// its text input; type the name from the modal title, then click Delete.
	const heading = page.getByText(/^Delete Role "(.+)"$/);
	await expect(heading).toBeVisible({ timeout: 5_000 });
	const text = await heading.innerText();
	const match = text.match(/^Delete Role "(.+)"$/);
	const name = match?.[1] ?? '';
	const modalForm = page.locator('form').filter({ hasText: /Delete Role/ });
	await modalForm.locator('input[type="text"]').fill(name);
	const submit = modalForm.getByRole('button', { name: /^(Delete|Deleting\.\.\.)$/ });
	await expect(submit).toBeEnabled({ timeout: 5_000 });
	await submit.click();
}

export async function expectToast(page: Page, matcher: RegExp, timeout = 10_000): Promise<void> {
	await expect(page.getByText(matcher).first()).toBeVisible({ timeout });
}

// Edit-role page (edit.tsx + permission-picker.tsx).

// "Clear All" sits opposite the section label and empties the whole permission
// map, which re-disables Save Changes.
export async function clickClearAllPermissions(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Clear All$/ }).click();
}

// The UnsavedChangesGuard modal, shown when navigating away from the edit tab
// with pending edits. Buttons are "Stay" (dismiss) and "Leave" (discard + go).
export async function expectUnsavedChangesModal(page: Page): Promise<void> {
	await expect(page.getByText(/^Unsaved changes$/)).toBeVisible({ timeout: 5_000 });
}
