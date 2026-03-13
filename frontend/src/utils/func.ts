import { Accessor, JSX } from "solid-js";
import { Color } from "./color";
import { ActionTypes, ResourceTypes } from "./types";

const resourceActionMap: Record<ResourceTypes, ActionTypes[]> = {
	billing: ["view", "edit", "makePayment"],
	containerRegistryRepository: ["create", "edit", "delete", "view", "push", "pull", "deleteImage"],
	database: ["view", "edit", "create", "delete", "backup", "restore"],
	deployment: ["view", "edit", "create", "delete", "start", "stop"],
	dnsRecord: ["view", "edit", "add", "delete"],
	domain: ["view", "add", "verify", "delete"],
	managedUrl: ["view", "edit", "delete", "add", "verify"],
	runner: ["view", "edit", "create", "delete", "regenerateToken"],
	secret: ["view", "edit", "create", "delete"],
	staticSite: ["view", "edit", "create", "delete", "upload", "start", "stop"],
	volume: ["create", "delete", "view", "edit"],
	viewRoles: [],
	modifyRoles: [],
	editWorkspace: [],
};

const resourceTypes = [
	"billing",
	"containerRegistryRepository",
	"database",
	"deployment",
	"dnsRecord",
	"domain",
	"managedUrl",
	"runner",
	"secret",
	"staticSite",
	"volume",
	"viewRoles",
	"modifyRoles",
	"editWorkspace",
];

const userActionTypes = [
	"view",
	"edit",
	"makePayment",
	"create",
	"delete",
	"push",
	"pull",
	"deleteImage",
	"backup",
	"restore",
	"start",
	"stop",
	"add",
	"verify",
	"regenerateToken",
	"upload",
];

const get = <T>(v: T | Accessor<T>): T => (typeof v === "function" ? (v as Accessor<T>)() : v);

const Jsx = (element: JSX.Element) => {
	return () => element;
};

function Uuid(value: string) {
	return value.replaceAll("-", "");
}

function variantBgClass(styleVariant: string) {
	switch (styleVariant) {
		case "light":
			return "bg-secondary-light";
		case "medium":
			return "bg-secondary-medium";
		case "dark":
			return "bg-secondary-dark";
		default:
			return "bg-secondary-light";
	}
}

const getColorClasses = (color: Color) => {
	switch (color) {
		case Color.Primary:
			return {
				border: "border-primary",
				hoverBorder: "enabled:hover:border-primary",
				text: "text-primary",
				hoverText: "enabled:hover:text-primary",
				hoverBg: "enabled:hover:bg-primary",
				bg: "bg-primary",
			};
		case Color.Secondary:
			return {
				border: "border-secondary",
				hoverBorder: "enabled:hover:border-secondary",
				text: "text-secondary",
				hoverText: "enabled:hover:text-secondary",
				hoverBg: "enabled:hover:bg-secondary",
				bg: "bg-secondary",
			};
		case Color.Grey:
			return {
				border: "border-grey",
				hoverBorder: "enabled:hover:border-grey",
				text: "text-grey",
				hoverText: "enabled:hover:text-grey",
				hoverBg: "enabled:hover:bg-grey",
				bg: "bg-grey",
			};
		case Color.Success:
			return {
				border: "border-success",
				hoverBorder: "enabled:hover:border-success",
				text: "text-success",
				hoverText: "enabled:hover:text-success",
				hoverBg: "enabled:hover:bg-success",
				bg: "bg-success",
			};
		case Color.Error:
			return {
				border: "border-error",
				hoverBorder: "enabled:hover:border-error",
				text: "text-error",
				hoverText: "enabled:hover:text-error",
				hoverBg: "enabled:hover:bg-error",
				bg: "bg-error",
			};
		case Color.Warning:
			return {
				border: "border-warning",
				hoverBorder: "enabled:hover:border-warning",
				text: "text-warning",
				hoverText: "enabled:hover:text-warning",
				hoverBg: "enabled:hover:bg-warning",
				bg: "bg-warning",
			};
		case Color.Black:
			return {
				border: "border-black",
				hoverBorder: "enabled:hover:border-black",
				text: "text-black",
				hoverText: "enabled:hover:text-black",
				hoverBg: "enabled:hover:bg-black",
				bg: "bg-black",
			};
		case Color.White:
			return {
				border: "border-white",
				hoverBorder: "enabled:hover:border-white",
				text: "text-white",
				hoverText: "enabled:hover:text-white",
				hoverBg: "enabled:hover:bg-white",
				bg: "bg-white",
			};
		case Color.Info:
			return {
				border: "border-info",
				hoverBorder: "enabled:hover:border-info",
				text: "text-info",
				hoverText: "enabled:hover:text-info",
				hoverBg: "enabled:hover:bg-info",
				bg: "bg-info",
			};
		case Color.Disabled:
			return {
				border: "border-disabled",
				hoverBorder: "enabled:hover:border-disabled",
				text: "text-disabled",
				hoverText: "enabled:hover:text-disabled",
				hoverBg: "enabled:hover:bg-disabled",
				bg: "bg-disabled",
			};
		default:
			return { border: "", text: "", hoverBg: "" };
	}
};
// Helper to parse permission names like "deployment::view" into { resourceType: "deployment", action: "view" }
const parsePermissionName = (name: string) => {
	const parts = name.split("::");
	return {
		resourceType: parts[0] || "",
		action: parts[1] || "",
	};
};

const parseCamelCase = (str: string) => {
	return str.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/^./, (char) => char.toUpperCase());
};

const safelyParseJSON = <T>(jsonString: string): T | undefined => {
	try {
		return JSON.parse(jsonString) as T;
	} catch (error) {
		console.error("Error parsing JSON string:", error);
		return undefined;
	}
};

// Map resource types to their API endpoints
const getResourceEndpoint = (type: string) => {
	const endpointMap: Record<string, string> = {
		deployment: "deployment",
		containerRegistry: "container-registry",
		runner: "runner",
		staticSite: "static-site",
		volume: "volume",
		database: "database",
		secret: "secret",
		domain: "domain",
		mangagedUrl: "managed-url",
	};
	return endpointMap[type];
};

const convertFileToBase64 = (file: File): Promise<string> => {
	return new Promise((resolve, reject) => {
		const reader = new FileReader();

		reader.onload = () => {
			const result = reader.result;

			if (result && typeof result === "string") {
				// Remove the data URL prefix to get just the base64 string
				const b64String = result.replace(/^data:.+;base64,/, "");

				resolve(b64String);
			} else {
				reject("Failed to read file");
			}
		};
		reader.onerror = () => reject("Error reading file");
		reader.readAsDataURL(file);
	});
};

/**
 * Parses a date string or Date object into a valid Date.
 * Handles formats like "2026-02-06 21:54:25.712709 +00:00:00" from the API.
 */
const parseDate = (date: Date | string): Date | null => {
	if (date instanceof Date) {
		return isNaN(date.getTime()) ? null : date;
	}

	// Handle format like "2026-02-06 21:54:25.712709 +00:00:00"
	// Convert to ISO format: replace space with 'T' and fix timezone
	const isoString = date.replace(" ", "T").replace(/\s*\+(\d{2}):(\d{2}):\d{2}$/, "+$1:$2");
	const parsed = new Date(isoString);

	return isNaN(parsed.getTime()) ? null : parsed;
};

/**
 * Formats a date as a relative time string (e.g., "just now", "5 minutes ago").
 */
const formatRelativeTime = (date: Date | string): string => {
	const d = parseDate(date);

	if (!d) {
		return "Unknown";
	}

	const now = new Date();
	const diffMs = now.getTime() - d.getTime();
	const diffSec = Math.floor(diffMs / 1000);
	const diffMin = Math.floor(diffSec / 60);
	const diffHour = Math.floor(diffMin / 60);
	const diffDay = Math.floor(diffHour / 24);

	if (diffSec < 60) return "Just now";
	if (diffMin < 60) return `${diffMin} minute${diffMin !== 1 ? "s" : ""} ago`;
	if (diffHour < 24) return `${diffHour} hour${diffHour !== 1 ? "s" : ""} ago`;
	if (diffDay < 30) return `${diffDay} day${diffDay !== 1 ? "s" : ""} ago`;

	return d.toLocaleDateString();
};

const formatSize = (bytes: bigint | string | undefined): string => {
	if (!bytes) return "0 B";

	const numBytes = typeof bytes === "bigint" ? Number(bytes) : Number(bytes);

	if (isNaN(numBytes) || numBytes === 0) return "0 B";

	const units = ["B", "KB", "MB", "GB", "TB", "PB"];
	const k = 1024;
	const i = Math.floor(Math.log(numBytes) / Math.log(k));

	const size = (numBytes / Math.pow(k, i)).toFixed(2);
	return `${size} ${units[i]}`;
};

/**
 * Formats a date for display in tooltips with a shorter format.
 */
const formatDate = (date: Date | string): string => {
	const d = parseDate(date);

	if (!d) {
		return "N/A";
	}

	return d.toLocaleString("en-US", {
		year: "numeric",
		month: "short",
		day: "numeric",
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
	});
};

export {
	get,
	Jsx,
	Uuid,
	variantBgClass,
	getColorClasses,
	parseCamelCase,
	parsePermissionName,
	getResourceEndpoint,
	convertFileToBase64,
	safelyParseJSON,
	resourceActionMap,
	resourceTypes,
	userActionTypes,
	parseDate,
	formatRelativeTime,
	formatSize,
	formatDate,
};
