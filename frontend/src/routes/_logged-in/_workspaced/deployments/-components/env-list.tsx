import { FiLock, FiTrash2, FiUpload } from "solid-icons/fi";
import { debounce } from "@solid-primitives/scheduled";
import { createEffect, createMemo, createSignal, Show } from "solid-js";
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
	/**
	 * Fires when a row's value has just been converted into a secret. It only
	 * takes effect once the deployment is saved, which is the parent's to say.
	 */
	onSecretCreated?: () => void;
	/** Label for the left gutter. See `EnvInput`'s `label`. */
	label?: MaybeAccessor<string>;
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
 * The deployment's environment variables, plus everything that acts on them
 * beyond plain editing: the .env upload, the hints for values that look like
 * credentials, and converting one into a secret. `EnvInput` below stays a plain
 * key/value editor.
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

	// Only plain, non-empty values can become secrets, so only they are linted.
	const convertible = createMemo(() =>
		Object.entries(current())
			.filter(([key, value]) => key !== "" && typeof value === "string" && value !== "")
			.map(([key, value]) => ({ key, value: value as string }))
	);

	// Secret names are unique per workspace; the convert modal warns when the
	// name typed is taken, and a key that already names a secret gets no hint.
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
	const lintDebounced = debounce((entries: ReturnType<typeof convertible>) => {
		void Promise.all(
			entries.map(async (entry) => [entry.key, await lintEnvVar(entry.key, entry.value)] as const)
		).then((results) => setFindings(Object.fromEntries(results)));
	}, LINT_DEBOUNCE_MS);

	createEffect(() => {
		const entries = convertible();
		if (isServer || get(props.disabled)) {
			lintDebounced.clear();
			return;
		}
		lintDebounced(entries);
	});

	/**
	 * What to say about a row, or `undefined` to stay quiet. A key that already
	 * names a secret can't be converted, so there is nothing to offer there.
	 */
	const hintFor = (key: string, value: EnvironmentVariableValue): string | undefined => {
		if (typeof value !== "string" || key === "" || value === "") return undefined;
		if (existingSecretNames().has(key.toLowerCase())) return undefined;

		// secretlint names what it found ("OpenAI API token"), which beats our
		// generic wording whenever it has an opinion.
		const found = findings()[key]?.[0];
		if (found) return `${found.charAt(0).toUpperCase()}${found.slice(1)} - visible to viewers. Store as secret?`;

		return looksLikeSecret(key, value) ? "Possible secret - visible to viewers. Convert to secret?" : undefined;
	};

	const [convertOpen, setConvertOpen] = createSignal(false);
	const [converting, setConverting] = createSignal<{ key: string; value: string } | null>(null);

	const openConvert = (key: string, value: string) => {
		setConverting({ key, value });
		setConvertOpen(true);
	};

	const applyConvertedSecret = (key: string, secretId: string) => {
		pushIntoEditor({ ...current(), [key]: { fromSecret: secretId } });
		props.onSecretCreated?.();
	};

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
				label={props.label}
				secrets={() =>
					(secretsQuery.data?.secrets ?? []).map((secret) => ({ id: secret.id, name: secret.name }))
				}
				rowHint={(row) => (
					<Show when={!get(props.disabled) ? hintFor(row.key, row.value) : undefined}>
						{(hint) => (
							// Spans the key and value columns, with invisible copies of the toggle
							// and delete button so the hint ends where the value input does. The
							// copies are a full row tall, so the hint sits at the top of its row,
							// and the negative margin gives back the empty space below it.
							<div class="flex items-start gap-4 w-full -mb-1.5">
								<div class="flex-12 flex items-center justify-end gap-3 min-w-0">
									<Alert type="warning" message={hint()} hideIcon />
									<Button
										type="button"
										variant={ButtonVariant.Plain}
										onClick={() => openConvert(row.key, row.value as string)}
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
				{/* Same two-column split as the rows above, so this lines up with
				    the inputs rather than the label gutter. */}
				<div class="flex gap-8 w-full">
					<Show when={get(props.label)}>
						<div class="flex-2" />
					</Show>
					<div class="flex-10 flex items-center gap-8">
						<Button
							type="button"
							variant={ButtonVariant.Plain}
							onClick={() => setUploadOpen(true)}
							class="flex items-center gap-2 text-sm cursor-pointer"
						>
							<FiUpload size={14} />
							Upload your .env file
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
					entry={converting}
					existingSecretNames={existingSecretNames}
					onConverted={applyConvertedSecret}
				/>
			</Show>
		</div>
	);
};

export default EnvList;
