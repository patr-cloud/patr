/// <reference types="@solidjs/start/env" />
interface ViteTypeOptions {
	// By adding this line, you can make the type of ImportMetaEnv strict
	// to disallow unknown keys.
	strictImportMetaEnv: unknown;
}

interface ImportMetaEnv {
	readonly VITE_BASE_URL: string;
	readonly VITE_TURNSTILE_SITE_KEY: string;
	readonly VITE_CLOUD_MODE: string;
	readonly VITE_BASE_DOMAIN: string;
	readonly VITE_DEPLOYMENT_DOMAIN: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
