import { defineConfig } from "oxfmt";

export default defineConfig({
	arrowParens: "avoid",
	semi: true,
	singleQuote: false,
	trailingComma: "all",
	printWidth: 120,
	useTabs: true,
	tabWidth: 2,
	endOfLine: "lf",
	sortPackageJson: {
		sortScripts: true,
	},
	sortImports: {
		newlinesBetween: false,
		groups: [
			["value-builtin", "value-external"],
			["value-internal", "value-parent", "value-sibling", "value-index"],
			"type-import",
			"unknown",
		],
	},
	ignorePatterns: ["**/node_modules/**", "**/dist/**", "**/.raycast/**", "**/raycast-env.d.ts"],
	overrides: [
		{
			files: ["**/*.toml"],
			options: {
				useTabs: false,
			},
		},
	],
});
