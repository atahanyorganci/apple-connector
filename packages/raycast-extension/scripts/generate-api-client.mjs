#!/usr/bin/env node
// Generate `src/lib/api.gen.ts` from the committed OpenAPI contract.
//
// `docs/openapi.json` is itself generated (`cargo run -p apple-connector --bin
// export-openapi`) and byte-compared in CI, so the client types are a pure
// function of it. Output is deterministic: schemas and operations are emitted in
// sorted order, and property order follows the contract.
//
// Usage:
//   node scripts/generate-api-client.mjs           # write
//   node scripts/generate-api-client.mjs --check   # fail if stale
//
// Zero dependencies by design — the workspace is on TypeScript 7 (the native
// port), and the usual generators drive the TypeScript 5 compiler API.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "../../..");
const SPEC = resolve(ROOT, "docs/openapi.json");
const OUT = resolve(HERE, "../src/lib/api.gen.ts");

const HTTP_METHODS = ["get", "put", "post", "patch", "delete", "head"];
const IDENT = /^[A-Za-z_$][A-Za-z0-9_$]*$/;

/** Last path segment of a `#/components/schemas/X` pointer. */
function refName(ref) {
	const name = ref.split("/").pop();
	if (!name) {
		throw new Error(`unsupported $ref: ${ref}`);
	}
	return name;
}

function quoteProp(name) {
	return IDENT.test(name) ? name : JSON.stringify(name);
}

function literal(value) {
	return typeof value === "string" ? JSON.stringify(value) : String(value);
}

function union(parts) {
	const seen = [...new Set(parts)];
	if (seen.length === 0) {
		return "never";
	}
	return seen.length === 1 ? seen[0] : seen.join(" | ");
}

/** JSDoc block for a schema description, indented to `indent`. */
function docComment(schema, indent) {
	const text = schema?.description;
	if (!text) {
		return "";
	}
	const lines = String(text).trim().split("\n");
	if (lines.length === 1) {
		return `${indent}/** ${lines[0]} */\n`;
	}
	return `${indent}/**\n${lines.map(l => `${indent} * ${l}`.trimEnd()).join("\n")}\n${indent} */\n`;
}

/** Map a scalar JSON Schema `type` to its TypeScript equivalent. */
function scalar(type, schema, indent) {
	switch (type) {
		case "string":
			return schema.enum ? union(schema.enum.map(literal)) : "string";
		case "integer":
		case "number":
			return schema.enum ? union(schema.enum.map(literal)) : "number";
		case "boolean":
			return "boolean";
		case "null":
			return "null";
		case "array":
			return `${wrap(tsType(schema.items ?? {}, indent))}[]`;
		case "object":
			return objectType(schema, indent);
		default:
			throw new Error(`unsupported type: ${String(type)}`);
	}
}

/** Parenthesise a union before applying `[]`, so `A | B` becomes `(A | B)[]`. */
function wrap(type) {
	return type.includes("|") || type.includes("&") ? `(${type})` : type;
}

function objectType(schema, indent = "") {
	const props = schema.properties;
	if (!props || Object.keys(props).length === 0) {
		return "Record<string, never>";
	}
	const required = new Set(schema.required ?? []);
	const inner = `${indent}\t`;
	const fields = Object.entries(props).map(([name, prop]) => {
		const optional = required.has(name) ? "" : "?";
		return `${docComment(prop, inner)}${inner}${quoteProp(name)}${optional}: ${tsType(prop, inner)};`;
	});
	return `{\n${fields.join("\n")}\n${indent}}`;
}

function tsType(schema, indent = "") {
	if (schema === true || schema === undefined) {
		return "unknown";
	}
	if (schema.$ref) {
		return refName(schema.$ref);
	}
	if (schema.oneOf) {
		return union(schema.oneOf.map(s => tsType(s, indent)));
	}
	if (schema.anyOf) {
		return union(schema.anyOf.map(s => tsType(s, indent)));
	}
	if (schema.allOf) {
		const parts = schema.allOf.map(s => tsType(s, indent));
		return parts.length === 1 ? parts[0] : parts.map(wrap).join(" & ");
	}
	if (schema.enum && !schema.type) {
		return union(schema.enum.map(literal));
	}
	if (Array.isArray(schema.type)) {
		return union(schema.type.map(t => scalar(t, { ...schema, type: t }, indent)));
	}
	if (schema.type) {
		return scalar(schema.type, schema, indent);
	}
	// A schema with properties but no explicit `type` is still an object.
	return schema.properties ? objectType(schema, indent) : "unknown";
}

/** Build an object type from a list of OpenAPI parameters. */
function paramsType(params, indent) {
	if (params.length === 0) {
		return "never";
	}
	const inner = `${indent}\t`;
	const fields = params
		.map(p => {
			const optional = p.required ? "" : "?";
			const doc = docComment(p, inner);
			return `${doc}${inner}${quoteProp(p.name)}${optional}: ${tsType(p.schema ?? {}, inner)};`;
		})
		.join("\n");
	return `{\n${fields}\n${indent}}`;
}

/**
 * Response body type for a content map, plus the runtime `kind` the client uses
 * to pick between `.json()`, `.arrayBuffer()` and `.text()`.
 */
function contentType(content, indent) {
	if (!content) {
		return { type: "void", kind: "void" };
	}
	const json = content["application/json"];
	if (json) {
		return { type: tsType(json.schema ?? {}, indent), kind: "json" };
	}
	if (content["application/octet-stream"]) {
		// `ArrayBuffer`, not `Blob`: the extension targets `lib: ES2023` with no DOM,
		// and attachment bytes get written to a temp file via `response.arrayBuffer()`.
		return { type: "ArrayBuffer", kind: "binary" };
	}
	// text/plain, text/markdown, text/calendar, text/vcard, application/*+xml
	const first = Object.keys(content)[0];
	if (first && (first.startsWith("text/") || first.endsWith("+xml"))) {
		return { type: "string", kind: "text" };
	}
	return { type: "unknown", kind: "json" };
}

/** Lowest 2xx response is the success case. */
function successResponse(op, indent) {
	const codes = Object.keys(op.responses ?? {})
		.filter(c => /^2\d\d$/.test(c))
		.toSorted();
	const code = codes[0];
	if (!code) {
		return { code: "200", type: "void", kind: "void" };
	}
	return { code, ...contentType(op.responses[code].content, indent) };
}

function main() {
	const check = process.argv.includes("--check");
	const spec = JSON.parse(readFileSync(SPEC, "utf8"));

	const out = [];
	out.push("/**");
	out.push(" * Generated from `docs/openapi.json` — do not edit by hand.");
	out.push(" *");
	out.push(" * Regenerate with `pnpm generate` after updating the contract via");
	out.push(" * `cargo run -p apple-connector --bin export-openapi docs/openapi.json`.");
	out.push(" *");
	out.push(` * API version: ${spec.info?.version ?? "unknown"}`);
	out.push(" */");
	out.push("");

	// --- Schemas -------------------------------------------------------------
	const schemas = spec.components?.schemas ?? {};
	for (const name of Object.keys(schemas).toSorted()) {
		const schema = schemas[name];
		out.push(docComment(schema, "").trimEnd());
		out.push(`export type ${name} = ${tsType(schema, "")};`);
		// A plain string enum also gets a runtime value list, so callers can
		// validate untrusted input and build pickers without restating the set.
		if (schema.type === "string" && Array.isArray(schema.enum)) {
			const values = schema.enum.map(v => JSON.stringify(v)).join(", ");
			out.push(`export const ${name}Values = [${values}] as const satisfies readonly ${name}[];`);
		}
		out.push("");
	}

	// --- Operations ----------------------------------------------------------
	const operations = [];
	for (const path of Object.keys(spec.paths).toSorted()) {
		const item = spec.paths[path];
		for (const method of HTTP_METHODS) {
			const op = item[method];
			if (!op) {
				continue;
			}
			const params = op.parameters ?? [];
			const byLocation = loc => params.filter(p => p.in === loc);
			const body = op.requestBody?.content?.["application/json"]?.schema;
			const success = successResponse(op, "\t\t");
			operations.push({
				id: op.operationId,
				summary: op.summary,
				method: method.toUpperCase(),
				path,
				pathParams: paramsType(byLocation("path"), "\t\t"),
				query: paramsType(byLocation("query"), "\t\t"),
				headers: paramsType(byLocation("header"), "\t\t"),
				body: body ? tsType(body, "\t\t") : "never",
				status: success.code,
				response: success.type,
				kind: success.kind,
			});
		}
	}
	const sorted = operations.toSorted((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));

	out.push("/** Every operation in the contract, keyed by `operationId`. */");
	out.push("export interface Operations {");
	for (const op of sorted) {
		out.push(docComment({ description: op.summary }, "\t").trimEnd());
		out.push(`\t${quoteProp(op.id)}: {`);
		out.push(`\t\tmethod: ${JSON.stringify(op.method)};`);
		out.push(`\t\tpath: ${JSON.stringify(op.path)};`);
		out.push(`\t\tpathParams: ${op.pathParams};`);
		out.push(`\t\tquery: ${op.query};`);
		out.push(`\t\theaders: ${op.headers};`);
		out.push(`\t\tbody: ${op.body};`);
		out.push(`\t\tstatus: ${op.status};`);
		out.push(`\t\tresponse: ${op.response};`);
		out.push("\t};");
	}
	out.push("}");
	out.push("");
	out.push("/** Union of every `operationId` in the contract. */");
	out.push("export type OperationId = keyof Operations;");
	out.push("");
	out.push("/** How the client should decode a successful response body. */");
	out.push('export type ResponseKind = "json" | "text" | "binary" | "void";');
	out.push("");
	out.push("/** Runtime method, path template and decoding strategy for each operation. */");
	out.push("export const routes = {");
	for (const op of sorted) {
		const path = JSON.stringify(op.path);
		out.push(
			`\t${quoteProp(op.id)}: { method: ${JSON.stringify(op.method)}, path: ${path}, kind: ${JSON.stringify(op.kind)} },`,
		);
	}
	out.push("} as const satisfies Record<OperationId, { method: string; path: string; kind: ResponseKind }>;");
	out.push("");

	const text = `${out
		.join("\n")
		.replace(/\n{3,}/g, "\n\n")
		.trimEnd()}\n`;

	if (check) {
		let current = "";
		try {
			current = readFileSync(OUT, "utf8");
		} catch {
			console.error("src/lib/api.gen.ts is missing. Run `pnpm generate`.");
			process.exit(1);
		}
		if (current !== text) {
			console.error("src/lib/api.gen.ts is stale. Run `pnpm generate` and commit the result.");
			process.exit(1);
		}
		console.log(`api.gen.ts is up to date (${operations.length} operations, ${Object.keys(schemas).length} schemas)`);
		return;
	}

	writeFileSync(OUT, text);
	console.log(`Wrote src/lib/api.gen.ts (${operations.length} operations, ${Object.keys(schemas).length} schemas)`);
}

main();
