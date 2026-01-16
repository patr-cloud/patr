const RESOURCE_NAME_REGEX = /^[a-zA-Z0-9\-_ \.]{4,255}$/g;
const PHONE_NUMBER_REGEX = /^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$/g;
const USERNAME_VALIDITY_REGEX = /^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$/g;

// Pattern strings for HTML input pattern attribute (without delimiters and flags)
const RESOURCE_NAME_PATTERN = "[a-zA-Z0-9\\-_ \\.]{4,255}";
const PHONE_NUMBER_PATTERN = "\\(?\\d{3}\\)?[-.\\s]?\\d{3}[-.\\s]?\\d{4}";
const USERNAME_VALIDITY_PATTERN = "[a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]";

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
	const specialChars = new Set([
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

export {
	RESOURCE_NAME_REGEX,
	PHONE_NUMBER_REGEX,
	USERNAME_VALIDITY_REGEX,
	RESOURCE_NAME_PATTERN,
	PHONE_NUMBER_PATTERN,
	USERNAME_VALIDITY_PATTERN,
};
