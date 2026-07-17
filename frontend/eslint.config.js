import js from "@eslint/js";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import solidPlugin from "eslint-plugin-solid";
import prettierConfig from "eslint-config-prettier";
import globals from "globals";

export default [
	{
		ignores: [
			"dist/**",
			"node_modules/**",
			".vinxi/**",
			".output/**",
			"*.config.ts",
			"*.config.js",
			"src/routeTree.gen.ts",
		],
	},
	js.configs.recommended,
	{
		files: ["**/*.{ts,tsx,js,jsx}"],
		languageOptions: {
			parser: tsParser,
			parserOptions: {
				ecmaVersion: "latest",
				sourceType: "module",
				project: "./tsconfig.json",
			},
			globals: {
				...globals.browser,
				...globals.es2021,
				...globals.node,
			},
		},
		plugins: {
			"@typescript-eslint": tsPlugin,
			solid: solidPlugin,
		},
		rules: {
			...tsPlugin.configs.recommended.rules,
			...solidPlugin.configs.typescript.rules,
			"max-len": [
				"warn",
				{
					code: 120,
					tabWidth: 2,
					ignoreUrls: true,
					ignoreStrings: true,
					ignoreTemplateLiterals: true,
					ignoreRegExpLiterals: true,
				},
			],
			indent: ["error", "tab", { SwitchCase: 1 }],
			"no-tabs": "off",
			// Solid refs (`let el!: HTMLDivElement; <div ref={el}>`) are assigned by
			// the runtime through the ref binding, which this rule (newly enabled in
			// @eslint/js v10's recommended set) can't see and flags as never assigned.
			"no-unassigned-vars": "off",
			"@typescript-eslint/no-unused-vars": [
				"warn",
				{
					argsIgnorePattern: "^_",
					varsIgnorePattern: "^_",
				},
			],
			"@typescript-eslint/no-explicit-any": "warn",
			"@typescript-eslint/explicit-module-boundary-types": "off",
			"solid/reactivity": "warn",
			"solid/no-destructure": "warn",
		},
	},
	prettierConfig,
];
