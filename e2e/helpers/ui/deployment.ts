import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/deployments/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/deployments/new.tsx (create)
//   frontend/src/routes/_logged-in/_workspaced/deployments/$id.tsx
//       (detail: metrics / info / environment / logs)
//   .../deployments/-components/{info,metrics,logs,port,probe-input,config-mount}.tsx
//   .../deployments/-components/{environment,env-list,env-input,env-convert-modal}.tsx
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
	// List renders a mobile card grid and a desktop table (both in the DOM);
	// scope to the table so the name matches a single element at 1280 viewport.
	return page.getByRole('table').getByText(name, { exact: true });
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

// ---------- .env upload modal (env-upload-modal.tsx) ----------

// Opens the review modal and hands it `contents` as a .env file. The file input
// is hidden behind a dropzone, so set it directly; `name="env-file"` keeps this
// off the config-mount FileInputs ("deployment-config") on the same page.
export async function uploadEnvFile(page: Page, contents: string): Promise<void> {
	await page.getByRole('button', { name: /Upload your \.env file/i }).click();
	await page.locator('input[name="env-file"]').setInputFiles({
		name: '.env',
		mimeType: 'text/plain',
		buffer: Buffer.from(contents),
	});
}

// "Parsed N variables from .env" summary shown once a file is ingested.
export function envUploadSummary(page: Page) {
	return page.getByText(/Parsed\s+\d+\s+variables?\s+from/i);
}

// Key inputs of the parsed rows, in order ("KEY" placeholder). Assert contents
// with toHaveValue: the value is a DOM property, not a matchable attribute.
export function envUploadKeys(page: Page) {
	return page.locator('input[placeholder="KEY"]');
}

export function addToDeploymentButton(page: Page) {
	return page.getByRole('button', { name: /Add to deployment/i });
}

export async function submitEnvUpload(page: Page): Promise<void> {
	await addToDeploymentButton(page).click();
}

// ---------- Convert to secrets (env-list.tsx + env-convert-modal.tsx) ----------

// The per-row hint shown when a value looks like a credential. The wording is
// secretlint's own ("found Stripe secret key") when a vendor rule matches, and
// "<KEY> looks like a secret" when only the local heuristics do.
export function envSecretHint(page: Page, text: RegExp) {
	return page.getByText(text);
}

// Opens the modal for every convertible row. There is a second button with the
// same label inside the modal (the submit), so this is scoped to the one that
// is on the page before the modal exists.
export async function openConvertToSecrets(page: Page): Promise<void> {
	await page.getByRole('button', { name: /Convert to secrets/i }).click();
}

// The per-row "Convert" button beside a hint, which opens the modal with just
// that row ticked.
export async function convertSingleEnv(page: Page): Promise<void> {
	await page
		.getByRole('button', { name: /^Convert$/ })
		.first()
		.click();
}

export function convertModalHeading(page: Page) {
	return page.getByText('Convert to secrets', { exact: true });
}

export function convertEmptyState(page: Page) {
	return page.getByText(/No environment variables to convert/i);
}

// The Checkbox component keeps its real <input> `sr-only`, so Playwright sees
// it as hidden and `.check()` would never pass actionability. Clicking the
// wrapping <label> is both what a user does and what actually toggles it.
const checkboxLabels = (page: Page) => page.locator('label:has(input[type="checkbox"])');

export function convertSelectAll(page: Page) {
	return checkboxLabels(page).filter({ hasText: 'Select all' });
}

/** One row's checkbox, indexed past the "Select all" that precedes them. */
export function convertRowCheckbox(page: Page, row = 0) {
	return checkboxLabels(page).filter({ hasNotText: 'Select all' }).nth(row);
}

// Row controls inside the modal. The name input opens blank; the value is
// seeded from the deployment's row.
export function convertNameInput(page: Page) {
	return page.locator('input[placeholder="Secret name"]');
}

export function convertValueInput(page: Page) {
	return page.locator('input[placeholder="Secret value"]');
}

export function convertNameRequiredError(page: Page) {
	return page.getByText('Give this secret a name', { exact: true });
}

// The modal's submit. Scoped to the dialog's footer by taking the last match,
// since the page behind it has a button with the same label.
export function convertSubmitButton(page: Page) {
	return page.getByRole('button', { name: /^(Convert to secrets|Converting…)$/ }).last();
}

// The String / Secret toggle on each row (value-type-toggle.tsx).
export function valueTypeToggle(page: Page, type: 'String' | 'Secret', row = 0) {
	return page.getByRole('radio', { name: type, exact: true }).nth(row);
}

// The secret picker a row shows once its toggle is on Secret.
export function envSecretPicker(page: Page) {
	return page.locator('input[placeholder="Select a secret"]');
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

// Environment variables moved off the info tab onto their own (environment.tsx).
export function environmentTab(page: Page) {
	return page.getByRole('button', { name: 'Configuration', exact: true });
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
