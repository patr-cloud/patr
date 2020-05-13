module.exports = {
    root: true,
    parser: '@typescript-eslint/parser',
    plugins: [
        '@typescript-eslint',
    ],
    extends: [
        'airbnb-typescript/base'
    ],
    rules: {
        "@typescript-eslint/indent": [2, "tab"],
		"no-tabs": 0,
		"@typescript-eslint/no-unused-vars": 0,
		"no-param-reassign": ["error", { "props": false }]
    },
    parserOptions: {
        project: './tsconfig.json'
    },
};
