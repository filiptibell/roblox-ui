import * as vscode from "vscode";
import * as path from "path";
import * as cp from "child_process";
import * as semver from "semver";

export type SourcemapNode = {
	name: string;
	className: string;
	filePaths?: string[];
	children?: SourcemapNode[];
	parent?: SourcemapNode;
};

const sourcemapSetParents = (
	node: SourcemapNode,
	parent: SourcemapNode | undefined
) => {
	if (parent) {
		node.parent = parent;
	}
	if (node.children) {
		for (const child of node.children.values()) {
			sourcemapSetParents(child, node);
		}
	}
};

export const parseSourcemap = (txt: string): SourcemapNode => {
	const sourcemap = JSON.parse(txt);
	sourcemapSetParents(sourcemap, undefined);
	return sourcemap;
};

export const getSourcemapNodeTreeOrder = (node: SourcemapNode): number => {
	if (
		node.className === "Script" ||
		node.className === "LocalScript" ||
		node.className === "ModuleScript"
	) {
		return 3;
	} else if (node.className === "Folder") {
		return 2;
	} else {
		return 1;
	}
};

export const findFilePath = (
	workspaceRoot: string,
	node: SourcemapNode
): [string | null, boolean | null] => {
	if (node.filePaths) {
		let filePath = node.filePaths.find((filePath) => {
			return (
				filePath.endsWith(".project.json") ||
				filePath.endsWith(".lua") ||
				filePath.endsWith(".luau")
			);
		});
		if (filePath) {
			return [
				path.join(workspaceRoot, filePath),
				!filePath.endsWith(".project.json"),
			];
		}
	}
	return [null, null];
};

const globalWatchSupportCache: Map<string, boolean> = new Map();
export const rojoSourcemapWatchIsSupported = (cwd: string) => {
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
