import { FiLock, FiTrash2, FiUpload } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, onCleanup, Show } from "solid-js";
import { isServer } from "solid-js/web";
import { EnvironmentVariableValue } from "~/bindings";
import { Alert, Button, ButtonVariant } from "~/components";
import { useSecretsQuery } from "~/hooks/fetch";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { lintEnvVar } from "~/utils/secret-lint";
import { MaybeAccessor } from "~/utils/types";
import EnvConvertModal from "./env-convert-modal";
import EnvInput from "./env-input";
import EnvUploadModal from "./env-upload-modal";
import ValueTypeToggle from "./value-type-toggle";

interface EnvListProps {
	/** Current environment variables (source of truth). */
	value: MaybeAccessor<Record<string, EnvironmentVariableValue>>;
	/** Fires whenever the committed (validated) map changes. */
	onChange: (next: Record<string, EnvironmentVariableValue>) => void;
	/** Fires whenever the validity of the rows changes. Parents use this to gate submit. */
	onValidityChange?: (valid: boolean) => void;
	/** Disables all inputs. */
	disabled?: MaybeAccessor<boolean>;
	/** Additional class for the root container. */
	class?: MaybeAccessor<string>;
}

/** How long the rows must settle before secretlint is asked about them. */
const LINT_DEBOUNCE_MS = 300;

// Names that almost always hold a credential. `KEY` on its own is left out on
// purpose — `SORT_KEY` and `PARTITION_KEY` are ordinary values, and a hint that
// cries wolf gets ignored.
const SECRET_KEY_PATTERN =
	/(SECRET|TOKEN|PASSWORD|PASSWD|PWD|CREDENTIAL|PRIVATE_KEY|API_KEY|APIKEY|ACCESS_KEY|ENCRYPTION_KEY|SIGNING|AUTH|SALT|DSN)/i;

// Values that look random enough to be a credential but match no known shape:
// long, from a base64/hex alphabet, and mixing letters with digits.
const looksHighEntropy = (value: string): boolean =>
	value.length >= 24 && /^[A-Za-z0-9+/=_-]+$/.test(value) && /[A-Za-z]/.test(value) && /\d/.test(value);

// Ordinary configuration that would otherwise trip the entropy check.
const isPlainConfig = (value: string): boolean =>
	/^\d+$/.test(value) || // ports, counts
	/^(true|false)$/i.test(value) ||
	/^v?\d+\.\d+/.test(value) || // versions
	value.includes("/") || // paths
	value.includes(" ");

const isSecretValue = (value: EnvironmentVariableValue): boolean =>
	typeof value === "object" && value !== null && "fromSecret" in value;

/**
 * The generic half of the check, run on every render.
 *
 * Vendor token shapes are secretlint's job (see `lintEnvVar`); these are the
 * cases it deliberately won't flag — a value that is only suspicious because of
 * what the key is called (`PASSWORD=hunter2`), or because it looks random.
 */
const looksLikeSecret = (key: string, value: string): boolean => {
	if (SECRET_KEY_PATTERN.test(key)) return true;
	if (isPlainConfig(value)) return false;
	return looksHighEntropy(value);
};

/**
 * The deployment's environment variables, plus everything that acts on them as
 * a set: the .env upload, the convert-to-secrets flow, and the hints for values
 * that look like credentials. `EnvInput` below stays a plain key/value editor.
 */
const EnvList = (props: EnvListProps) => {
	// What the editor seeds its rows from. This must NOT follow the editor's own
	// output: the committed map leaves out invalid rows (a duplicate key, say),
	// so feeding it back would delete the row the user is still typing in. It
	// only changes when the parent hands down a new map, or when we push one in.
	// Seeded by the effect below, which runs before the editor first renders.
	const [editorValue, setEditorValue] = createSignal<Record<string, EnvironmentVariableValue>>({});

	// Mirrors what the editor currently holds, so the hints and the conversion
	// see the user's edits and not just the map the parent last handed down.
	const [edited, setEdited] = createSignal<Record<string, EnvironmentVariableValue> | null>(null);

	createEffect(() => {
		setEditorValue(get(props.value) ?? {});
		setEdited(null);
	});

	const current = (): Record<string, EnvironmentVariableValue> => edited() ?? editorValue();

	const handleChange = (next: Record<string, EnvironmentVariableValue>) => {
		setEdited(next);
		props.onChange(next);
	};

	// Replaces what the editor is showing. Only for changes made outside the
	// rows themselves — a conversion or a .env upload.
	const pushIntoEditor = (next: Record<string, EnvironmentVariableValue>) => {
		setEditorValue(next);
		setEdited(next);
		props.onChange(next);
	};

	// Only plain, non-empty values can become secrets.
	const convertible = createMemo(() =>
		Object.entries(current())
			.filter(([key, value]) => key !== "" && typeof value === "string" && value !== "")
			.map(([key, value]) => ({ key, value: value as string }))
	);

	// Secret names are unique per workspace, so a key that already names one
	// can't be converted — the modal flags those rows.
	const secretsQuery = useSecretsQuery(
		() => undefined,
		() => "100"
	);

	const existingSecretNames = createMemo(
		() => new Set((secretsQuery.data?.secrets ?? []).map((secret) => secret.name.toLowerCase()))
	);

	// secretlint's findings, keyed by env var name. Filled in behind the sync
	// check below, so the hint never waits on it.
	const [findings, setFindings] = createSignal<Record<string, string[]>>({});

	// Linting is async and the preset is a lazy chunk, so it runs on a trailing
	// debounce rather than on every keystroke. Only plain values are linted —
	// a value that is already a secret reference has nothing to find.
	createEffect(() => {
		const entries = convertible();
		if (isServer || get(props.disabled)) return;

		const timer = setTimeout(() => {
			void Promise.all(
				entries.map(async (entry) => [entry.key, await lintEnvVar(entry.key, entry.value)] as const)
			).then((results) => setFindings(Object.fromEntries(results)));
		}, LINT_DEBOUNCE_MS);

		onCleanup(() => clearTimeout(timer));
	});

	/**
	 * What to say about a row, or `undefined` to stay quiet. A key that already
	 * names a secret can't be converted, so there is nothing to offer there.
	 */
	const hintFor = (key: string, value: EnvironmentVariableValue): string | undefined => {
		if (typeof value !== "string" || key === "" || value === "") return undefined;
		if (existingSecretNames().has(key.toLowerCase())) return undefined;

		// secretlint names what it found ("Stripe secret key detected"), which
		// beats our generic wording whenever it has an opinion.
		const found = findings()[key];
		if (found?.length) return found[0];

		return looksLikeSecret(key, value) ? `${key} looks like a secret` : undefined;
	};

	const [convertOpen, setConvertOpen] = createSignal(false);
	const [preselected, setPreselected] = createSignal<string[]>([]);

	const openConvert = (keys: string[]) => {
		setPreselected(keys);
		setConvertOpen(true);
	};

	const applyConvertedSecrets = (converted: Record<string, EnvironmentVariableValue>) =>
		pushIntoEditor({ ...current(), ...converted });

	const [uploadOpen, setUploadOpen] = createSignal(false);

	// Keys the upload modal validates against: one already bound to a secret
	// must not be silently replaced with a plain value from a .env file.
	const existingKeys = createMemo(
		() => new Map(Object.entries(current()).map(([key, value]) => [key, isSecretValue(value)]))
	);

	// Spreading keeps the existing keys in place and appends the new ones.
	const applyUploadedEnvs = (entries: Array<{ key: string; value: string }>) =>
		pushIntoEditor({ ...current(), ...Object.fromEntries(entries.map((entry) => [entry.key, entry.value])) });

	return (
		<div class="flex flex-col gap-1 w-full">
			<EnvInput
				value={editorValue}
				onChange={handleChange}
				onValidityChange={props.onValidityChange}
				disabled={props.disabled}
				class={props.class}
				secrets={() =>
					(secretsQuery.data?.secrets ?? []).map((secret) => ({ id: secret.id, name: secret.name }))
				}
				rowHint={(row) => (
					<Show when={!get(props.disabled) ? hintFor(row.key, row.value) : undefined}>
						{(hint) => (
							// Spans the key and value columns, with spacers standing in for the
							// toggle and delete button so the hint ends where the value input does.
							<div class="flex items-center gap-4 w-full">
								<div class="flex-12 flex items-center justify-end gap-3 min-w-0">
									<Alert type="warning" message={hint()} truncate />
									<Button
										type="button"
										variant={ButtonVariant.Plain}
										onClick={() => openConvert([row.key])}
										class="flex items-center gap-2 text-sm whitespace-nowrap cursor-pointer"
									>
										<FiLock size={14} />
										Convert
									</Button>
								</div>
								<div class="invisible flex" aria-hidden="true">
									<ValueTypeToggle value={() => "string"} onChange={() => {}} />
								</div>
								<Button
									type="button"
									variant={ButtonVariant.Outlined}
									class="flex-1 flex items-center gap-2 invisible"
									color={Color.Error}
								>
									<FiTrash2 size={16} />
								</Button>
							</div>
						)}
					</Show>
				)}
			/>

			<Show when={!get(props.disabled)}>
				<div class="flex flex-col gap-1 w-full">
					<div class="flex items-center gap-8">
						<Button
							type="button"
							variant={ButtonVariant.Plain}
							onClick={() => setUploadOpen(true)}
							class="flex items-center gap-2 text-sm cursor-pointer"
						>
							<FiUpload size={14} />
							Upload your .env file
						</Button>
						{/* Deliberately not disabled when there is nothing to convert: the
						    modal says so, which beats a dead button that explains nothing. */}
						<Button
							type="button"
							variant={ButtonVariant.Plain}
							onClick={() => openConvert([])}
							class="flex items-center gap-2 text-sm cursor-pointer"
						>
							<FiLock size={14} />
							Convert to secrets
						</Button>
					</div>
				</div>

				<EnvUploadModal
					isOpen={uploadOpen}
					setIsOpen={setUploadOpen}
					existingKeys={existingKeys}
					onSubmit={applyUploadedEnvs}
				/>

				<EnvConvertModal
					isOpen={convertOpen}
					setIsOpen={setConvertOpen}
					convertible={convertible}
					existingSecretNames={existingSecretNames}
					initialSelection={preselected}
					onConverted={applyConvertedSecrets}
				/>
			</Show>
		</div>
	);
};

export default EnvList;
