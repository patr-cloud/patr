const RESOURCE_NAME_REGEX = /^[a-zA-Z0-9\-_ .]{4,255}$/;
const PHONE_NUMBER_REGEX = /^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$/;
const USERNAME_VALIDITY_REGEX = /^[a-z0-9_][a-z0-9_.-]*[a-z0-9_]$/;
// Pattern matches USER_NAME_REGEX on the backend: 1–100 chars, no HTML
// metacharacters (<, >, &), no control chars / tabs / newlines.
// eslint-disable-next-line no-control-regex
const USER_NAME_REGEX = /^[^<>&\n\r\t\x00-\x1f]{1,100}$/;
// Pattern matches ROLE_DESCRIPTION_REGEX on the backend (same shape, 0–500
// chars, empty allowed).
// eslint-disable-next-line no-control-regex
const ROLE_DESCRIPTION_REGEX = /^[^<>&\n\r\t\x00-\x1f]{0,500}$/;
// The XSS-relevant chars + control chars, used as a substring test for inline
// form errors that fire on each keystroke.
// eslint-disable-next-line no-control-regex
const XSS_PATTERN = /[<>&\x00-\x1f]/;

// Pattern strings for HTML input pattern attribute (without delimiters and flags)
const RESOURCE_NAME_PATTERN = "[a-zA-Z0-9\\-_ \\.]{4,255}";
const PHONE_NUMBER_PATTERN = "\\(?\\d{3}\\)?[-.\\s]?\\d{3}[-.\\s]?\\d{4}";
const USERNAME_VALIDITY_PATTERN = "[a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]";
// HTML input pattern for "username or email" — matches either the username
// shape or any string containing `@` and `.`. The backend's
// `validate_username_or_email` does the authoritative check; this is a
// best-effort client gate.
const USERNAME_OR_EMAIL_PATTERN = "([a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]|[^@\\s]+@[^@\\s]+\\.[^@\\s]+)";

// Special characters accepted in passwords. Mirrors the backend
// `validate_password` set. Shared between `validatePassword` and
// `passwordStrength` so the two never drift.
const PASSWORD_SPECIAL_CHARS = new Set([
	"@",
	"!",
	"#",
	"$",
	"%",
	"^",
	"&",
	"*",
	"?",
	"/",
	"\\",
	"|",
	"~",
	"`",
	".",
	",",
	";",
	":",
	"<",
	">",
	"[",
	"]",
	"{",
	"}",
]);

/**
 * Validates if a password meets the following requirements:
 * - A minimum of 8 characters
 * - Must contain at least one digit
 * - One uppercase letter
 * - One lowercase letter
 * - One special character (!@#$%^&*?/\|~`.,:;<>[]{}  etc.)
 *
 * @param value - The password string to validate
 * @returns An object with `valid` boolean and optional `error` message
 */
export function validatePassword(value: string): {
	valid: boolean;
	error?: string;
} {
	let hasDigit = false;
	let hasUppercase = false;
	let hasLowercase = false;
	let hasSpecial = false;

	for (const char of value) {
		if (/\d/.test(char)) hasDigit = true;
		if (/[A-Z]/.test(char)) hasUppercase = true;
		if (/[a-z]/.test(char)) hasLowercase = true;
		if (PASSWORD_SPECIAL_CHARS.has(char)) hasSpecial = true;
	}

	if (!hasDigit) {
		return { valid: false, error: "Password must contain at least one digit" };
	}

	if (!hasLowercase) {
		return {
			valid: false,
			error: "Password must contain at least one lowercase",
		};
	}

	if (!hasUppercase) {
		return {
			valid: false,
			error: "Password must contain at least one uppercase",
		};
	}

	if (!hasSpecial) {
		return {
			valid: false,
			error: "Password must contain at least one special character",
		};
	}

	return { valid: true };
}

export interface PasswordRequirement {
	label: string;
	met: boolean;
}

export interface PasswordStrengthResult {
	/** Overall tier. `weak` = fails `validatePassword`. */
	tier: "weak" | "fair" | "good" | "strong";
	/** Number of the 4 meter segments that should be filled. */
	segments: number;
	/** Theme color token for the tier. */
	color: "error" | "warning" | "success";
	/** Per-rule met/unmet state for the checklist. */
	requirements: PasswordRequirement[];
}

/**
 * Grades a password for the strength indicator. The tier is anchored to
 * `validatePassword`: anything that fails it is `weak`. Beyond that:
 *
 * - When a zxcvbn `score` (0–4) is supplied, it drives the tier: 0–1 = fair,
 *   2–3 = good, 4 = strong. This catches guessable-but-well-formed passwords
 *   (e.g. `Password@123`) that a length check alone would rate "strong".
 * - When `score` is omitted (the estimator hasn't lazy-loaded yet), it falls
 *   back to a length heuristic (< 12 = good, >= 12 = strong) so the meter still
 *   works on the first keystrokes.
 *
 * In both paths a password shorter than 8 characters is capped at `fair`, since
 * the backend rejects it for length regardless of composition.
 *
 * The requirements list surfaces every individual rule (including the 8-char
 * minimum, which `validatePassword` itself does not check).
 */
export function passwordStrength(value: string, score?: number): PasswordStrengthResult {
	let hasDigit = false;
	let hasUppercase = false;
	let hasLowercase = false;
	let hasSpecial = false;

	for (const char of value) {
		if (/\d/.test(char)) hasDigit = true;
		if (/[A-Z]/.test(char)) hasUppercase = true;
		if (/[a-z]/.test(char)) hasLowercase = true;
		if (PASSWORD_SPECIAL_CHARS.has(char)) hasSpecial = true;
	}

	const requirements: PasswordRequirement[] = [
		{ label: "At least 8 characters", met: value.length >= 8 },
		{ label: "One uppercase letter", met: hasUppercase },
		{ label: "One lowercase letter", met: hasLowercase },
		{ label: "One number", met: hasDigit },
		{ label: "One special character", met: hasSpecial },
	];

	if (!validatePassword(value).valid) {
		return { tier: "weak", segments: 1, color: "error", requirements };
	}
	// Never advertise more than "fair" for a password the backend rejects for length.
	if (value.length < 8) {
		return { tier: "fair", segments: 2, color: "warning", requirements };
	}

	if (score !== undefined) {
		if (score <= 1) {
			return { tier: "fair", segments: 2, color: "warning", requirements };
		}
		if (score <= 3) {
			return { tier: "good", segments: 3, color: "warning", requirements };
		}
		return { tier: "strong", segments: 4, color: "success", requirements };
	}

	// Estimator not loaded yet — length-based fallback.
	if (value.length < 12) {
		return { tier: "good", segments: 3, color: "warning", requirements };
	}
	return { tier: "strong", segments: 4, color: "success", requirements };
}

/**
 * Validates a person's first or last name. Returns an error message if the
 * value is empty / whitespace-only, contains HTML metacharacters (<, >, &),
 * contains control characters / newlines / tabs, or is over 100 characters.
 * Mirrors the backend's USER_NAME_REGEX preprocess.
 */
export function validateNameField(value: string): string | undefined {
	if (!value || value.trim().length === 0) return "Required";
	if (XSS_PATTERN.test(value)) {
		return "Names cannot contain <, >, &, or control characters";
	}
	if (value.length > 100) return "Name must be 100 characters or fewer";
	return undefined;
}

/**
 * Validates a role description. Empty is allowed (backend substitutes a
 * default). Returns an error message for HTML metacharacters or strings over
 * 500 characters.
 */
export function validateRoleDescription(value: string): string | undefined {
	if (XSS_PATTERN.test(value)) {
		return "Description cannot contain <, >, &, or control characters";
	}
	if (value.length > 500) {
		return "Description must be 500 characters or fewer";
	}
	return undefined;
}

/**
 * Validates a login identifier as either a username or an email. Phone-shape
 * input is rejected — phone login is not exposed in the UI this round.
 * Mirrors the backend's `validate_username_or_email`.
 */
export function validateUsernameOrEmail(value: string): string | undefined {
	if (!value || value.trim().length === 0) return "Required";
	if (value.length < 4) return "Must be at least 4 characters";
	if (value.includes("@")) {
		if (!value.includes(".") || value.length < 5) {
			return "Not a valid email address";
		}
		return undefined;
	}
	if (!USERNAME_VALIDITY_REGEX.test(value)) {
		return "Must be a valid username or email";
	}
	return undefined;
}

export {
	RESOURCE_NAME_REGEX,
	PHONE_NUMBER_REGEX,
	USERNAME_VALIDITY_REGEX,
	USER_NAME_REGEX,
	ROLE_DESCRIPTION_REGEX,
	XSS_PATTERN,
	RESOURCE_NAME_PATTERN,
	PHONE_NUMBER_PATTERN,
	USERNAME_VALIDITY_PATTERN,
	USERNAME_OR_EMAIL_PATTERN,
};
