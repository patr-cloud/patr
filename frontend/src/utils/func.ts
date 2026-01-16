import { Accessor, JSX } from "solid-js";

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
	parseCamelCase,
	parsePermissionName,
	getResourceEndpoint,
	convertFileToBase64,
};
