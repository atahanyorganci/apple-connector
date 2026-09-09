import { environment } from "@raycast/api";
import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ApiError, ServerUnreachableError } from "./errors";

function cacheDir(): string {
	// `environment.supportPath` is per-extension and survives between launches;
	// fall back to the system temp dir when it is unavailable.
	return environment.supportPath || join(tmpdir(), "apple-connector-raycast");
}

/**
 * Fetch bytes to a file and return its path.
 *
 * Raycast's `Detail` and `List` cannot render bytes from a stream — they take a
 * file path or URL — so anything binary has to land on disk first. The filename
 * is derived from the cache key, so repeat views reuse one file.
 *
 * Takes a URL rather than an operation id because not every byte-serving
 * endpoint declares a media type in the contract: `/v1/contacts/{id}/photo`
 * documents a 200 with no `content`, so its generated response type is `void`.
 * Build the URL with `urlFor` to keep the path itself contract-derived.
 */
export async function fetchToFile(url: string, cacheKey: string, extension = "bin"): Promise<string> {
	const directory = cacheDir();
	await mkdir(directory, { recursive: true });

	const digest = createHash("sha256").update(cacheKey).digest("hex").slice(0, 32);
	const path = join(directory, `${digest}.${extension}`);

	let response: Response;
	try {
		response = await fetch(url);
	} catch (cause) {
		throw new ServerUnreachableError(url, cause);
	}
	if (!response.ok) {
		throw await ApiError.fromResponse(response);
	}

	const bytes = await response.arrayBuffer();
	await writeFile(path, Buffer.from(bytes));
	return path;
}
