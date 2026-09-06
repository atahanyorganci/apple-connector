import { defineConfig } from "oxlint";

export default defineConfig({
	plugins: ["typescript", "unicorn", "oxc", "react", "import"],
	categories: {
		correctness: "error",
		suspicious: "error",
		perf: "warn",
	},
	options: {
		typeAware: true,
		typeCheck: true,
		denyWarnings: true,
		reportUnusedDisableDirectives: "error",
	},
	settings: {
		react: {
			version: "19.0.0",
		},
	},
	rules: {
		curly: "error",
		eqeqeq: ["error", "always"],
		"typescript/consistent-type-imports": [
			"error",
			{
				disallowTypeAnnotations: true,
				fixStyle: "separate-type-imports",
			},
		],
		"typescript/no-floating-promises": "error",
		"import/no-cycle": "error",
		"react/self-closing-comp": "error",
	},
	ignorePatterns: ["**/node_modules/**", "**/dist/**", "**/raycast-env.d.ts"],
	overrides: [
		{
			files: ["src/**/*.{ts,tsx}"],
			env: {
				browser: true,
			},
		},
	],
});
