// Source of truth for client-side input validation. Mirrors the rules
// enforced by the patr Rust backend (models/src/utils/mod.rs and the
// per-domain *.rs files under api/src/db). Keep these in sync when the
// backend changes — the backend is the last line of defense, this is
// just so the UI can fail fast.

export type Validation = { valid: true } | { valid: false; error: string };

const ok: Validation = { valid: true };
const fail = (error: string): Validation => ({ valid: false, error });

// --- Regexes ---------------------------------------------------------------

// 4–255 chars; letters, digits, spaces, `- _ .`
const RESOURCE_NAME_REGEX = /^[a-zA-Z0-9\-_ .]{4,255}$/;
const RESOURCE_NAME_PATTERN = "[a-zA-Z0-9\\-_ \\.]{4,255}";

// lowercase start/end, lowercase/digit/`_`/`.`/`-` in middle, length >=2
const USERNAME_VALIDITY_REGEX = /^[a-z0-9_][a-z0-9_.-]*[a-z0-9_]$/;
const USERNAME_VALIDITY_PATTERN = "[a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]";

const PHONE_NUMBER_REGEX = /^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$/;
const PHONE_NUMBER_PATTERN = "\\(?\\d{3}\\)?[-.\\s]?\\d{3}[-.\\s]?\\d{4}";

const PHONE_COUNTRY_CODE_REGEX = /^[A-Z]{2}$/;

// 6 digits, optional middle hyphen (e.g. "123456" or "123-456")
const OTP_REGEX = /^\d{3}-?\d{3}$/;

// Basic RFC-5322-ish email check — same shape used by HTML5 type="email".
const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

// Single domain label (no dot). 2–63 chars per RFC 1035 / backend TLD rule.
const DOMAIN_LABEL_REGEX = /^([a-z0-9]|[a-z0-9][a-z0-9\-]*[a-z0-9])$/;
// Top-level domain, allows nested dots (e.g. "co.uk").
const DOMAIN_TLD_REGEX = /^([a-z0-9]|[a-z0-9][a-z0-9\-.]*[a-z0-9])$/;

// Managed-URL sub-domain: dot-separated lowercase labels, or literal "@".
const MANAGED_URL_SUBDOMAIN_REGEX =
	/^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$/;

// DNS record name: wildcard, wildcard-prefixed, dot-separated labels, or "@".
const DNS_RECORD_NAME_REGEX =
	/^((\*)|((\*\.)?(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])))$/;

// Container image name for external registries (e.g. docker.io).
const EXTERNAL_IMAGE_NAME_REGEX = /^[a-zA-Z0-9\-_ ./]{4,255}$/;

// Docker tag: lowercase letters, digits, `.`, `_`, `-`, non-empty, max 128.
const IMAGE_TAG_REGEX = /^[a-z0-9._-]{1,128}$/;

// Config-mount path inside a deployment container.
const CONFIG_MOUNT_PATH_REGEX = /^[a-zA-Z0-9_\-.()/]+$/;

// POSIX shell-style env var name.
const ENV_VAR_KEY_REGEX = /^[A-Za-z_][A-Za-z0-9_]*$/;
const ENV_VAR_KEY_MAX_LENGTH = 256;

// IPv4 with optional /CIDR (octet/CIDR bounds checked numerically below).
const IPV4_CIDR_REGEX = /^(\d{1,3}\.){3}\d{1,3}(\/\d{1,2})?$/;
// IPv6 with optional /CIDR — shape only; covered by numeric check.
const IPV6_CIDR_REGEX = /^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}(\/\d{1,3})?$/;

// --- Numeric bounds --------------------------------------------------------

const PORT_RANGE = { min: 1, max: 65535 } as const;
const HORIZONTAL_SCALE_RANGE = { min: 0, max: 256 } as const;
const PASSWORD_MIN_LENGTH = 8;

// --- Validators ------------------------------------------------------------

/** "4–255 chars; letters, digits, spaces, dash, underscore, dot only." */
const RESOURCE_NAME_RULE = "must be 4–255 characters using letters, digits, spaces, or - _ .";

function validateResourceName(value: string, field = "Name"): Validation {
	const v = value.trim();
	if (!v) return fail(`${field} is required.`);
	if (!RESOURCE_NAME_REGEX.test(v)) return fail(`${field} ${RESOURCE_NAME_RULE}`);
	return ok;
}

// Field-specific wrappers — same rule, friendlier message at the call site.
const validateWorkspaceName = (v: string) => validateResourceName(v, "Workspace name");
const validateDeploymentName = (v: string) => validateResourceName(v, "Deployment name");
const validateRunnerName = (v: string) => validateResourceName(v, "Runner name");
const validateRoleName = (v: string) => validateResourceName(v, "Role name");
const validateRepositoryName = (v: string) => validateResourceName(v, "Repository name");
const validateSecretName = (v: string) => validateResourceName(v, "Secret name");
const validateApiTokenName = (v: string) => validateResourceName(v, "Token name");

function validateUsername(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Username is required.");
	if (v.length < 2) return fail("Username must be at least 2 characters.");
	if (!USERNAME_VALIDITY_REGEX.test(v))
		return fail("Username may only contain lowercase letters, digits, `_`, `.`, `-`.");
	return ok;
}

function validateEmail(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Email is required.");
	if (!EMAIL_REGEX.test(v)) return fail("Enter a valid email address.");
	return ok;
}

function validateOtp(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Verification code is required.");
	if (!OTP_REGEX.test(v)) return fail("Enter the 6-digit code.");
	return ok;
}

function validatePersonName(value: string, field: string): Validation {
	const v = value.trim();
	if (!v) return fail(`${field} is required.`);
	if (v.length > 100) return fail(`${field} is too long.`);
	return ok;
}

const validateFirstName = (v: string) => validatePersonName(v, "First name");
const validateLastName = (v: string) => validatePersonName(v, "Last name");

function validateDomain(value: string): Validation {
	const v = value.trim().toLowerCase();
	if (!v) return fail("Domain is required.");
	if (v.length > 253) return fail("Domain is too long.");
	if (/^(https?:\/\/|www\.)|[/?#]/.test(value))
		return fail("Enter just the domain (no protocol, www, or path).");
	const labels = v.split(".");
	if (labels.length < 2) return fail("Enter a domain with a TLD (e.g. example.com).");
	for (const label of labels) {
		if (label.length < 1 || label.length > 63) return fail("Each domain label must be 1–63 chars.");
		if (!DOMAIN_LABEL_REGEX.test(label))
			return fail("Domain labels may only contain lowercase letters, digits and `-`.");
	}
	return ok;
}

function validateSubdomain(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Sub-domain is required.");
	if (v === "@") return ok;
	if (v !== v.toLowerCase()) return fail("Sub-domain must be lowercase.");
	if (!MANAGED_URL_SUBDOMAIN_REGEX.test(v))
		return fail("Sub-domain may only contain lowercase letters, digits, `_`, `-`, and `.`.");
	return ok;
}

function validateDnsRecordName(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Record name is required.");
	if (v === "@" || v === "*") return ok;
	if (v !== v.toLowerCase()) return fail("Record name must be lowercase.");
	if (!DNS_RECORD_NAME_REGEX.test(v)) return fail("Invalid DNS record name.");
	return ok;
}

function validateImageName(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Image name is required.");
	if (!EXTERNAL_IMAGE_NAME_REGEX.test(v))
		return fail("Image name must be 4–255 characters; letters, digits, spaces, `- _ . /` only.");
	return ok;
}

function validateImageTag(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Image tag is required.");
	if (v !== v.toLowerCase()) return fail("Image tag must be lowercase.");
	if (!IMAGE_TAG_REGEX.test(v))
		return fail("Image tag may only contain lowercase letters, digits, `.`, `_`, `-`.");
	return ok;
}

function validatePort(value: number | string): Validation {
	const n = typeof value === "string" ? Number(value) : value;
	if (!Number.isInteger(n)) return fail("Port must be a whole number.");
	if (n < PORT_RANGE.min || n > PORT_RANGE.max)
		return fail(`Port must be between ${PORT_RANGE.min} and ${PORT_RANGE.max}.`);
	return ok;
}

function validateScaleRange(min: number, max: number): Validation {
	if (!Number.isInteger(min) || !Number.isInteger(max))
		return fail("Scale values must be whole numbers.");
	if (min < HORIZONTAL_SCALE_RANGE.min || max > HORIZONTAL_SCALE_RANGE.max)
		return fail(`Scale must be between ${HORIZONTAL_SCALE_RANGE.min} and ${HORIZONTAL_SCALE_RANGE.max}.`);
	if (min > max) return fail("Min scale cannot exceed max scale.");
	return ok;
}

function validateConfigMountPath(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Mount path is required.");
	if (!v.startsWith("/")) return fail("Mount path must start with `/`.");
	if (!CONFIG_MOUNT_PATH_REGEX.test(v))
		return fail("Mount path may only contain letters, digits, `_`, `-`, `.`, `(`, `)`, `/`.");
	return ok;
}

function validateEnvVarKey(value: string): Validation {
	const v = value.trim();
	if (!v) return fail("Variable name is required.");
	if (v.length > ENV_VAR_KEY_MAX_LENGTH) return fail(`Variable name must be ≤ ${ENV_VAR_KEY_MAX_LENGTH} chars.`);
	if (!ENV_VAR_KEY_REGEX.test(v))
		return fail("Variable name must start with a letter or `_` and contain only letters, digits, `_`.");
	return ok;
}

/** IPv4/IPv6 with optional CIDR. Used by ChipInput (validate prop). */
function validateIp(value: string): string | undefined {
	const v = value.trim();
	if (!v) return "IP is required.";

	if (IPV4_CIDR_REGEX.test(v)) {
		const [addr, cidr] = v.split("/");
		const octets = addr.split(".").map((o) => Number(o));
		if (octets.some((o) => o < 0 || o > 255)) return "Each IPv4 octet must be 0–255.";
		if (cidr !== undefined) {
			const c = Number(cidr);
			if (c < 0 || c > 32) return "IPv4 CIDR must be 0–32.";
		}
		return undefined;
	}

	if (IPV6_CIDR_REGEX.test(v)) {
		const [, cidr] = v.split("/");
		if (cidr !== undefined) {
			const c = Number(cidr);
			if (c < 0 || c > 128) return "IPv6 CIDR must be 0–128.";
		}
		return undefined;
	}

	return "Enter a valid IPv4 or IPv6 address (optionally with CIDR).";
}

function validateRequired(value: string, field: string): Validation {
	return value.trim() ? ok : fail(`${field} is required.`);
}

// Password keeps the bespoke checker (covers 4 character-class rules).
// Kept separate so callers can compose with other rules (length, match).
function validatePassword(value: string): Validation {
	if (!value) return fail("Password is required.");
	if (value.length < PASSWORD_MIN_LENGTH)
		return fail(`Password must be at least ${PASSWORD_MIN_LENGTH} characters.`);

	const specialChars = new Set([
		"@", "!", "#", "$", "%", "^", "&", "*", "?", "/", "\\", "|", "~",
		"`", ".", ",", ";", ":", "<", ">", "[", "]", "{", "}",
	]);

	let hasDigit = false;
	let hasUppercase = false;
	let hasLowercase = false;
	let hasSpecial = false;

	for (const char of value) {
		if (/\d/.test(char)) hasDigit = true;
		if (/[A-Z]/.test(char)) hasUppercase = true;
		if (/[a-z]/.test(char)) hasLowercase = true;
		if (specialChars.has(char)) hasSpecial = true;
	}

	if (!hasDigit) return fail("Password must contain at least one digit.");
	if (!hasLowercase) return fail("Password must contain at least one lowercase letter.");
	if (!hasUppercase) return fail("Password must contain at least one uppercase letter.");
	if (!hasSpecial) return fail("Password must contain at least one special character.");
	return ok;
}

export {
	// regexes / pattern strings (named to match historical imports)
	RESOURCE_NAME_REGEX,
	RESOURCE_NAME_PATTERN,
	USERNAME_VALIDITY_REGEX,
	USERNAME_VALIDITY_PATTERN,
	PHONE_NUMBER_REGEX,
	PHONE_NUMBER_PATTERN,
	PHONE_COUNTRY_CODE_REGEX,
	OTP_REGEX,
	EMAIL_REGEX,
	DOMAIN_LABEL_REGEX,
	DOMAIN_TLD_REGEX,
	MANAGED_URL_SUBDOMAIN_REGEX,
	DNS_RECORD_NAME_REGEX,
	EXTERNAL_IMAGE_NAME_REGEX,
	IMAGE_TAG_REGEX,
	CONFIG_MOUNT_PATH_REGEX,
	ENV_VAR_KEY_REGEX,
	IPV4_CIDR_REGEX,
	IPV6_CIDR_REGEX,
	// ranges
	PORT_RANGE,
	HORIZONTAL_SCALE_RANGE,
	PASSWORD_MIN_LENGTH,
	ENV_VAR_KEY_MAX_LENGTH,
	// validators
	validateResourceName,
	validateWorkspaceName,
	validateDeploymentName,
	validateRunnerName,
	validateRoleName,
	validateRepositoryName,
	validateSecretName,
	validateApiTokenName,
	validateUsername,
	validateEmail,
	validateOtp,
	validateFirstName,
	validateLastName,
	validatePersonName,
	validateDomain,
	validateSubdomain,
	validateDnsRecordName,
	validateImageName,
	validateImageTag,
	validatePort,
	validateScaleRange,
	validateConfigMountPath,
	validateEnvVarKey,
	validateIp,
	validateRequired,
	validatePassword,
};
