import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/workspace/roles/{index,new,$roleId}.tsx
//   .../roles/-components/{permission-selector,edit,users,role-header}.tsx

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

// PermissionSelector helpers — the InputDropdown options are plain <div>s in a
// Portal (no role="option"). We open the dropdown, click the option by text,
// then click the + button which has aria-label="Add Permission".
export async function addWorkspaceLevelPermission(
  page: Page,
  resourceLabel: 'Modify Roles' | 'View Roles' | 'Edit Workspace' | 'Billing',
): Promise<void> {
  const input = page.getByPlaceholder('Select Resource Type');
  await input.click();
  const option = page.getByText(resourceLabel, { exact: true });
  await option.first().waitFor({ state: 'visible', timeout: 5_000 });
  await option.first().click();
  // After selection the dropdown closes and the input shows the chosen label.
  await expect(input).toHaveValue(resourceLabel, { timeout: 3_000 });
  await page.getByRole('button', { name: 'Add Permission' }).click();
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
