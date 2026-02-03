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
				hoverBorder: "hover:border-primary",
				text: "text-primary",
				hoverText: "hover:text-primary",
				hoverBg: "hover:bg-primary",
				bg: "bg-primary",
			};
		case Color.Secondary:
			return {
				border: "border-secondary",
				hoverBorder: "hover:border-secondary",
				text: "text-secondary",
				hoverText: "hover:text-secondary",
				hoverBg: "hover:bg-secondary",
				bg: "bg-secondary",
			};
		case Color.Grey:
			return {
				border: "border-grey",
				hoverBorder: "hover:border-grey",
				text: "text-grey",
				hoverText: "hover:text-grey",
				hoverBg: "hover:bg-grey",
				bg: "bg-grey",
			};
		case Color.Success:
			return {
				border: "border-success",
				hoverBorder: "hover:border-success",
				text: "text-success",
				hoverText: "hover:text-success",
				hoverBg: "hover:bg-success",
				bg: "bg-success",
			};
		case Color.Error:
			return {
				border: "border-error",
				hoverBorder: "hover:border-error",
				text: "text-error",
				hoverText: "hover:text-error",
				hoverBg: "hover:bg-error",
				bg: "bg-error",
			};
		case Color.Warning:
			return {
				border: "border-warning",
				hoverBorder: "hover:border-warning",
				text: "text-warning",
				hoverText: "hover:text-warning",
				hoverBg: "hover:bg-warning",
				bg: "bg-warning",
			};
		case Color.Black:
			return {
				border: "border-black",
				hoverBorder: "hover:border-black",
				text: "text-black",
				hoverText: "hover:text-black",
				hoverBg: "hover:bg-black",
				bg: "bg-black",
			};
		case Color.White:
			return {
				border: "border-white",
				hoverBorder: "hover:border-white",
				text: "text-white",
				hoverText: "hover:text-white",
				hoverBg: "hover:bg-white",
				bg: "bg-white",
			};
		case Color.Info:
			return {
				border: "border-info",
				hoverBorder: "hover:border-info",
				text: "text-info",
				hoverText: "hover:text-info",
				hoverBg: "hover:bg-info",
				bg: "bg-info",
			};
		case Color.Disabled:
			return {
				border: "border-disabled",
				hoverBorder: "hover:border-disabled",
				text: "text-disabled",
				hoverText: "hover:text-disabled",
				hoverBg: "hover:bg-disabled",
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
};
