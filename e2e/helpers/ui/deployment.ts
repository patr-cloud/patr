import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/deployments/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/deployments/new.tsx (create)
//   frontend/src/routes/_logged-in/_workspaced/deployments/$id.tsx (detail: metrics/info/logs)
//   .../deployments/-components/{info,metrics,logs,port,env-input,probe-input,config-mount}.tsx
//
// Registry + runner pickers are InputDropdowns: an <input> (by placeholder) that
// opens a Portal of option <div>s (by label text). Selecting = click input,
// click option.

async function waitForVisible(page: Page, selector: string): Promise<void> {
  await page.locator(selector).first().waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

// ---------- List (/deployments) ----------

export async function openDeploymentList(page: Page): Promise<void> {
  await page.goto('/deployments', { waitUntil: 'domcontentloaded' });
}

export function emptyStateHeading(page: Page) {
  return page.getByText('No Deployments Added', { exact: true });
}

// "Create Deployment" link → /deployments/new (header button at >=1, empty-state
// CTA at 0).
export function createDeploymentLink(page: Page) {
  return page.getByRole('link', { name: /Create Deployment/i });
}

export function deploymentRow(page: Page, name: string) {
  return page.getByText(name, { exact: true });
}

// ---------- Create (/deployments/new) ----------

export async function openDeploymentCreate(page: Page): Promise<void> {
  await page.goto('/deployments/new', { waitUntil: 'domcontentloaded' });
  await waitForVisible(page, 'input[name="deployment-name"]');
}

export async function fillDeploymentName(page: Page, name: string): Promise<void> {
  await page.locator('input[name="deployment-name"]').fill(name);
}

// Pick an option from an InputDropdown identified by its placeholder. Clicks the
// input to open the Portal dropdown, then clicks the option by its visible label.
async function selectDropdownOption(page: Page, placeholder: string, label: string): Promise<void> {
  await page.locator(`input[placeholder="${placeholder}"]`).click();
  await page.getByText(label, { exact: true }).last().click();
}

// Registry options: "Patr Registry" | "Docker Hub".
export async function selectRegistry(
  page: Page,
  label: 'Patr Registry' | 'Docker Hub',
): Promise<void> {
  await selectDropdownOption(page, 'Select Registry', label);
}

// External (Docker Hub) image fields are plain text inputs.
export async function fillImageName(page: Page, image: string): Promise<void> {
  await page.locator('input[placeholder="Image Name"]').fill(image);
}

export async function fillImageTag(page: Page, tag: string): Promise<void> {
  await page.locator('input[placeholder="Image Tag"]').fill(tag);
}

export async function selectRunner(page: Page, runnerName: string): Promise<void> {
  await selectDropdownOption(page, 'Select Runner', runnerName);
}

export async function submitCreateDeployment(page: Page): Promise<void> {
  await page.getByRole('button', { name: /^(Create|Creating Deployment\.\.\.)$/ }).click();
}

// Port-row validation errors (port.tsx).
export function portError(
  page: Page,
  text: 'Must be a number' | 'Port out of range' | 'Duplicate port',
) {
  return page.getByText(text, { exact: true });
}

export async function fillFirstPort(page: Page, value: string): Promise<void> {
  await page.locator('input[placeholder="Enter Port Number"]').first().fill(value);
}

// Fill the first environment-variable row (env-input.tsx: "Enter Env Name" /
// "Enter Env Value" placeholders).
export async function fillFirstEnv(page: Page, key: string, value: string): Promise<void> {
  await page.locator('input[placeholder="Enter Env Name"]').first().fill(key);
  await page.locator('input[placeholder="Enter Env Value"]').first().fill(value);
}

// ---------- Detail (/deployments/{id}) ----------

export async function openDeploymentDetail(page: Page, id: string, tab?: string): Promise<void> {
  const suffix = tab === undefined ? '' : `?tab=${tab}`;
  await page.goto(`/deployments/${id}${suffix}`, { waitUntil: 'domcontentloaded' });
}

export function metricsTab(page: Page) {
  return page.getByRole('button', { name: 'Metrics', exact: true });
}

export function infoTab(page: Page) {
  return page.getByRole('button', { name: 'Info', exact: true });
}

export function logsTab(page: Page) {
  return page.getByRole('button', { name: 'Logs', exact: true });
}

// The Start (FiPlay) / Stop (FiPause) buttons are icon-only; locate them by
// their position in the header action row. Start shows only when stopped, Stop
// only when not-stopped. We expose count-based predicates rather than text.
export function noPermissionsHeading(page: Page) {
  return page.getByText("Can't View Resource", { exact: true });
}

export function noSuchTab(page: Page) {
  return page.getByText('No such tab', { exact: true });
}

// ---------- Info tab (update form) ----------

export function infoNameInput(page: Page) {
  return page.locator('input[name="deployment-name"]');
}

export function infoImageTagInput(page: Page) {
  return page.locator('input[placeholder="Image Tag"]');
}

export function updateButton(page: Page) {
  return page.getByRole('button', { name: /^(Update|Updating\.\.\.)$/ });
}

// ---------- Delete modal ----------

function deleteConfirm(page: Page) {
  return page.locator('button[type="submit"]', { hasText: /^Delete(ing\.\.\.)?$/ });
}

// Opens the delete modal from the detail header, types the deployment name to
// satisfy the name-match confirmation, and clicks confirm.
export async function deleteDeploymentViaModal(page: Page, name: string): Promise<void> {
  await page
    .getByRole('button', { name: /^Delete$/ })
    .first()
    .click();
  await page.getByText('Do You Really Want to Delete This Deployment?').waitFor({
    state: 'visible',
    timeout: HYDRATION_TIMEOUT,
  });
  await page.locator('input[type="text"]').last().fill(name);
  await expect(deleteConfirm(page)).toBeEnabled();
  await deleteConfirm(page).click();
}
