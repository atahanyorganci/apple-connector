import { environment } from "@raycast/api";
import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

function cacheDir(): string {
	// `environment.supportPath` is per-extension and survives between launches;
	// fall back to the system temp dir when it is unavailable.
	return environment.supportPath || join(tmpdir(), "apple-connector-raycast");
}

/**
 * Write bytes from a binary operation to a file and return its path.
 *
 * Raycast's `Detail` and `List` cannot render bytes from a stream — they take a
 * file path or URL — so anything binary has to land on disk first. The filename
 * is derived from the cache key, so repeat views reuse one file.
 */
export async function saveToFile(bytes: ArrayBuffer, cacheKey: string, extension = "bin"): Promise<string> {
	const directory = cacheDir();
	await mkdir(directory, { recursive: true });

	const digest = createHash("sha256").update(cacheKey).digest("hex").slice(0, 32);
	const path = join(directory, `${digest}.${extension}`);
	await writeFile(path, Buffer.from(bytes));
	return path;
}
