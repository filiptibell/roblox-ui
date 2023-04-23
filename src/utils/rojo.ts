import * as vscode from "vscode";
import * as cp from "child_process";
import * as semver from "semver";

import { SettingsManager } from "./settings";
import { SourcemapNode, parseSourcemap } from "./sourcemap";

const globalWatchSupportCache: Map<string, boolean> = new Map();
export const rojoSupportsSourcemapWatch = (cwd: string) => {
	const cached = globalWatchSupportCache.get(cwd);
	if (cached !== undefined) {
		return cached;
	}
	// Rojo version 7.3.0 is the minimum supported version for sourcemap watching
	let supported = true;
	const result = cp.spawnSync("rojo --version", {
		cwd: cwd,
		env: process.env,
		shell: true,
	});
	if (result.status !== null && result.status !== 0) {
		vscode.window.showWarningMessage(
			"Rojo Explorer failed to generate a sourcemap!" +
				"\nMake sure Rojo is installed and available in the current directory."
		);
		supported = false;
	} else {
		const version = result.stdout.toString("utf8").slice(5);
		if (!semver.satisfies(version, "^7.3.0")) {
			vscode.window.showWarningMessage(
				"Rojo Explorer failed to generate a sourcemap!" +
					`\nRojo is installed with version ${version}` +
					", but a minimum version of 7.3.0 is required."
			);
			supported = false;
		}
	}
	globalWatchSupportCache.set(cwd, supported);
	setTimeout(() => {
		globalWatchSupportCache.delete(cwd);
	}, 30_000);
	return supported;
};

export const rojoSourcemapWatch = (
	workspacePath: string,
	settings: SettingsManager,
	loadingCallback: (child: cp.ChildProcessWithoutNullStreams) => any,
	updateCallback: (
		child: cp.ChildProcessWithoutNullStreams,
		sourcemap: SourcemapNode
	) => any
): cp.ChildProcessWithoutNullStreams => {
	const updateArgs = [
		"sourcemap",
		"--watch",
		settings.get("rojoProjectFile") || "default.project.json",
		settings.get("includeNonScripts") ? "--include-non-scripts" : "",
	];

	const childProcess = cp.spawn("rojo", updateArgs, {
		cwd: workspacePath,
		env: process.env,
		shell: true,
	});

	// Listen for new sourcemaps being generated and output, here we will have to
	// keep track of stdout since data may be received in pieces and incomplete json
	// When we have complete parseable json we will update + clear the current stdout
	let stdout = "";
	loadingCallback(childProcess);
	childProcess.stdout.on("data", (data: Buffer) => {
		if (stdout === "") {
			loadingCallback(childProcess);
		}
		stdout += data.toString("utf8");
		try {
			const sourcemap = parseSourcemap(stdout);
			stdout = "";
			updateCallback(childProcess, sourcemap);
		} catch {}
	});

	// Listen for error messages and the child process closing
	let stderr = "";
	childProcess.stderr.on("data", (data: Buffer) => {
		stderr += data.toString("utf8");
	});
	childProcess.on("close", (code: number) => {
		if (childProcess.killed) {
			return;
		}
		if (code !== 0) {
			if (stderr.length > 0) {
				vscode.window.showErrorMessage(
					"Rojo Explorer failed to generate a sourcemap!" +
						`\nRojo exited with code ${code}` +
						`\nMessage:\n${stderr}`
				);
			} else {
				vscode.window.showErrorMessage(
					"Rojo Explorer failed to generate a sourcemap!" +
						`\nRojo exited with code ${code}`
				);
			}
		}
	});

	return childProcess;
};
