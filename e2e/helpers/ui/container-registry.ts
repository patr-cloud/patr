import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/container-registry/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/container-registry/new.tsx (create)
//   frontend/src/routes/_logged-in/_workspaced/container-registry/$id.tsx (detail)
//   .../container-registry/-components/{general,images}.tsx
//
// The create form's only field is #repository-name; the live registry-path
// preview shows once the trimmed name is non-empty.

async function waitForVisible(page: Page, selector: string): Promise<void> {
  await page.locator(selector).first().waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

// ---------- List (/container-registry) ----------

export async function openRegistryList(page: Page): Promise<void> {
  await page.goto('/container-registry', { waitUntil: 'domcontentloaded' });
}

export function emptyStateHeading(page: Page) {
  return page.getByText('No Container Repositories Yet', { exact: true });
}

// The "Create Repository" affordance is a link to /container-registry/new. It is
// the empty-state CTA at 0 repos and a header button at >=1 repos.
export function createRepoLink(page: Page) {
  return page.getByRole('link', { name: /Create Repository/i });
}

export function repoRow(page: Page, name: string) {
  // The list renders a mobile card grid (md:hidden) AND a desktop table
  // (hidden md:block) — both in the DOM. Scope to the table (the view shown at
  // Playwright's default 1280 viewport) so the name matches a single element.
  return page.getByRole('table').getByText(name, { exact: true });
}

// ---------- Create (/container-registry/new) ----------

export async function openRegistryCreate(page: Page): Promise<void> {
  await page.goto('/container-registry/new', { waitUntil: 'domcontentloaded' });
  await waitForVisible(page, '#repository-name');
}

export async function fillRepoName(page: Page, name: string): Promise<void> {
  await page.locator('#repository-name').fill(name);
}

export async function submitCreateRepo(page: Page): Promise<void> {
  await page.getByRole('button', { name: /^(Create Repository|Creating\.\.\.)$/ }).click();
}

// The live preview: "registry.patr.cloud/{ws}/{name}". Returns the locator so
// callers can assert visibility/text or its absence.
export function registryPathPreview(page: Page) {
  return page.getByText(/registry\.patr\.cloud\/[0-9a-f]+\//i).first();
}

export function nameErrorAlert(page: Page) {
  return page.getByText('Repository name is required.', { exact: true });
}

// ---------- Detail (/container-registry/{id}) ----------

export async function openRegistryDetail(page: Page, id: string, tab?: string): Promise<void> {
  const suffix = tab === undefined ? '' : `?tab=${tab}`;
  await page.goto(`/container-registry/${id}${suffix}`, { waitUntil: 'domcontentloaded' });
}

// HeadTab renders plain <button>s, not role="tab".
export function generalTab(page: Page) {
  return page.getByRole('button', { name: 'General', exact: true });
}

export function imagesTab(page: Page) {
  return page.getByRole('button', { name: 'Images', exact: true });
}

export function pushInstructionsHeading(page: Page) {
  return page.getByText('Push Instructions', { exact: true });
}

export function imagesEmptyState(page: Page) {
  return page.getByText('No Images Found', { exact: true });
}

// ---------- Delete modal ----------

// The detail-header delete trigger (default DeleteModal trigger button). Only
// rendered when the user has containerRegistryRepository::delete.
export function deleteTrigger(page: Page) {
  return page.getByRole('button', { name: /^Delete$/ });
}

// The modal's confirm submit (disabled until the typed name matches).
function deleteConfirm(page: Page) {
  return page.locator('button[type="submit"]', { hasText: /^Delete(ing\.\.\.)?$/ });
}

// Opens the delete modal from the detail header, types the repo name to satisfy
// the name-match confirmation, and clicks the confirm button.
export async function deleteRepoViaModal(page: Page, repoName: string): Promise<void> {
  await deleteTrigger(page).first().click();
  await page.getByText('Do You Really Want to Delete This Repository?').waitFor({
    state: 'visible',
    timeout: HYDRATION_TIMEOUT,
  });
  // The confirmation input is the only text input in the modal.
  await page.locator('input[type="text"]').last().fill(repoName);
  await expect(deleteConfirm(page)).toBeEnabled();
  await deleteConfirm(page).click();
}
