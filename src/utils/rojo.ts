import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import * as semver from "semver";

const fs = vscode.workspace.fs;

import { SettingsProvider } from "../providers/settings";
import { SourcemapNode } from "./sourcemap";
import { spawnWithTimeout } from "./child_process";

export type ProjectRootNode = {
	name: string;
	tree: ProjectFileNode;
};

export type ProjectFileNode = {
	[key: string]: any | ProjectFileNode;
	["$ignoreUnknownInstances"]?: boolean;
	["$className"]?: string;
	["$path"]?: string;
	// Extension-only properties, these don't actually exist in project files
	["$filePath"]?: string;
	["$folderPath"]?: string;
};

const ROJO_PROJECT_EXTENSION = ".project.json";
const ROJO_FILE_EXTENSIONS = [
	"meta.json",
	"model.json",
	"server.luau",
	"server.lua",
	"client.luau",
	"client.lua",
	"luau",
	"lua",
	"rbxm",
	"rbxmx",
	"rbxl",
	"rbxlx",
];

const globalWatchSupportCache: Map<string, boolean> = new Map();
export const rojoSupportsSourcemapWatch = async (cwd: string) => {
	const cached = globalWatchSupportCache.get(cwd);
	if (cached !== undefined) {
		return cached;
	}

	// Rojo version 7.3.0 is the minimum supported version for sourcemap watching
	let supported = true;
	const result = await spawnWithTimeout(
		"rojo",
		["--version"],
		{
			cwd: cwd,
			env: process.env,
			shell: true,
		},
		1_000
	);

	if (!result.ok) {
		vscode.window.showWarningMessage(
			"Failed to generate a sourcemap!" +
				"\nMake sure Rojo is installed and available in the current directory." +
				"\nThe extension will watch a 'sourcemap.json' file instead." +
				"\n\nError message: " +
				result.stderr
		);
		supported = false;
	} else {
		// Grab last "word" of string, meaning the last substring which
		// does not contain a space, supporting version strings like:
		// - Rojo 7.3.0
		// - Rojo v7.3.0
		// - Rojo (Forked Version) 7.3.0
		// - Rojo (Forked Version) v7.3.0
		const words = result.stdout.split(" ");
		const word = words[words.length - 1].trim();
		const version = word.startsWith("v") ? word.slice(1) : word;
		if (!semver.satisfies(version, "^7.3.0")) {
			vscode.window.showWarningMessage(
				"Failed to generate a sourcemap!" +
					`\nRojo is installed with version ${version}` +
					", but a minimum version of 7.3.0 is required." +
					"\nThe extension will watch a 'sourcemap.json' file instead."
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
	settings: SettingsProvider,
	callbacks: {
		loading: (child: cp.ChildProcessWithoutNullStreams) => any;
		errored: (
			child: cp.ChildProcessWithoutNullStreams,
			errorMessage: string
		) => any;
		update: (
			child: cp.ChildProcessWithoutNullStreams,
			sourcemap: SourcemapNode
		) => any;
	}
): cp.ChildProcessWithoutNullStreams => {
	const updateArgs = [
		"sourcemap",
		"--watch",
		settings.get("sourcemap.rojoProjectFile") || "default.project.json",
		settings.get("sourcemap.includeNonScripts")
			? "--include-non-scripts"
			: "",
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
	callbacks.loading(childProcess);
	childProcess.stdout.on("data", (data: Buffer) => {
		callbacks.loading(childProcess);
		stdout += data.toString("utf8");
		// Sourcemap to stdout always ends with a newline
		if (stdout.endsWith("\n")) {
			const sourcemap = JSON.parse(stdout);
			stdout = "";
			callbacks.update(childProcess, sourcemap);
		}
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
			const errorMessage =
				stderr.length > 0
					? "Failed to generate a sourcemap!" +
					  `\n\nRojo exited with code ${code}` +
					  "\n\nMessage:" +
					  `\n\n${stderr}`
					: "Failed to generate a sourcemap!" +
					  `\n\nRojo exited with code ${code}`;
			vscode.window.showErrorMessage(errorMessage);
			callbacks.errored(childProcess, `${stderr ?? ""}`);
		}
	});

	return childProcess;
};

export const extractRojoFileExtension = (filePath: string): string | null => {
	const fileName = path.basename(filePath);
	for (const ext of ROJO_FILE_EXTENSIONS) {
		if (fileName.endsWith(`.${ext}`)) {
			return ext;
		}
	}
	return null;
};

export const isProjectFilePath = (filePath: string): boolean => {
	return filePath.endsWith(ROJO_PROJECT_EXTENSION);
};

export const isBinaryFilePath = (filePath: string): boolean => {
	const fileExt = extractRojoFileExtension(filePath);
	return (
		fileExt === "rbxm" ||
		fileExt === "rbxmx" ||
		fileExt === "rbxl" ||
		fileExt === "rbxlx"
	);
};

export const isInitFilePath = (filePath: string): boolean => {
	if (isBinaryFilePath(filePath)) {
		return false;
	}
	const fileExt = extractRojoFileExtension(filePath);
	if (fileExt) {
		const fileName = path.basename(filePath, `.${fileExt}`);
		return fileName === "init";
	} else {
		return false;
	}
};

/**
 * Caches file system paths from project nodes, also making sure they exist.
 *
 * Does not modify project nodes without a `"$path"` attribute set.
 */
export const cacheProjectFileSystemPaths = async (
	workspacePath: string,
	projectPath: string,
	project: ProjectRootNode
) => {
	const rootAsNode = { [project.name]: project.tree };
	await cacheProjectFileSystemPathsForNode(workspacePath, rootAsNode);
	if (!project.tree["$filePath"]) {
		const relativeFilePath = projectPath.startsWith(workspacePath)
			? projectPath.slice(workspacePath.length + 1)
			: projectPath;
		if (relativeFilePath) {
			project.tree["$filePath"] = relativeFilePath;
		}
	}
};

const cacheProjectFileSystemPathsForNode = async (
	workspacePath: string,
	projectNode: ProjectFileNode,
	parent: ProjectFileNode | void
) => {
	const children: Map<string, ProjectFileNode> = new Map();
	for (const [key, value] of Object.entries(projectNode)) {
		if (!key.startsWith("$")) {
			children.set(key, value);
		}
	}

	await Promise.all(
		[...children.values()].map((child) =>
			cacheProjectFileSystemPathsForNode(
				workspacePath,
				child,
				projectNode
			)
		)
	);

	const nodePath = projectNode["$path"];
	if (nodePath && typeof nodePath === "string") {
		try {
			const fullPath = path.join(workspacePath, nodePath);
			const stats = await fs.stat(vscode.Uri.file(fullPath));
			const isFile = stats.type === vscode.FileType.File;
			if (isFile) {
				projectNode["$filePath"] = nodePath;
			}
			const isDir = stats.type === vscode.FileType.Directory;
			if (isDir) {
				projectNode["$folderPath"] = nodePath;
			}
			return;
		} catch {
			return;
		}
	}

	// We are at the root of the project, try to figure out some kind of shared
	// folder prefix based on the top level items, if we have a definite shared
	// folder prefix then we can enable features such as inserting services
	if (projectNode["$className"] === "DataModel") {
		let sharedPrefix: string | undefined;
		for (const child of children.values()) {
			const parentPath = child["$folderPath"]
				? path.join(child["$folderPath"], "..")
				: undefined;
			if (parentPath) {
				if (sharedPrefix) {
					if (!parentPath.startsWith(sharedPrefix)) {
						sharedPrefix = undefined;
						break;
					}
				} else {
					sharedPrefix = parentPath;
				}
			}
		}
		if (
			sharedPrefix &&
			sharedPrefix !== "." &&
			sharedPrefix !== "./" &&
			sharedPrefix !== "/"
		) {
			try {
				const fullPath = path.join(workspacePath, sharedPrefix);
				const stats = await fs.stat(vscode.Uri.file(fullPath));
				if (stats.type === vscode.FileType.Directory) {
					projectNode["$folderPath"] = sharedPrefix;
				}
			} catch {}
		}
	}
};

/**
 * Merges the given project file into the sourcemap.
 *
 * Does not modify the project file.
 *
 * This adds new `folderPath` properties to sourcemap nodes
 * which point to known folders on the filesystem, which is
 * something that the sourcemap does not contain by default.
 *
 * **NOTE:** Make sure to run `cacheProjectFileSystemPaths` on the
 * project before applying this merge, otherwise this will be a no-op.
 */
export const mergeProjectIntoSourcemap = (
	workspacePath: string,
	project: ProjectRootNode,
	sourcemap: SourcemapNode
) => {
	const rootAsNode = { [project.name]: project.tree };
	const sourcemapAsRoot = {
		className: "<<<ROOT>>>",
		name: "<<<ROOT>>>",
		children: [sourcemap],
	};
	mergeProjectNodeIntoSourcemapNode(
		workspacePath,
		rootAsNode,
		sourcemapAsRoot
	);
};

const mergeProjectNodeIntoSourcemapNode = (
	workspacePath: string,
	projectNode: ProjectFileNode,
	sourcemapNode: SourcemapNode
) => {
	const nodeFolderPath = projectNode["$folderPath"];
	if (nodeFolderPath) {
		sourcemapNode.folderPath = nodeFolderPath;
	}
	const nodeFilePath = projectNode["$filePath"];
	if (nodeFilePath) {
		if (sourcemapNode.filePaths) {
			if (!sourcemapNode.filePaths.find((p) => p === nodeFilePath)) {
				sourcemapNode.filePaths.push(nodeFilePath);
			}
		} else {
			sourcemapNode.filePaths = [nodeFilePath];
		}
	}
	if (sourcemapNode.children) {
		for (const [projectNodeName, projectNodeInner] of Object.entries(
			projectNode
		)) {
			if (!projectNodeName.startsWith("$")) {
				let sourcemapNodeInner;
				for (const child of sourcemapNode.children.values()) {
					if (child.name === projectNodeName) {
						sourcemapNodeInner = child;
						break;
					}
				}
				if (sourcemapNodeInner) {
					mergeProjectNodeIntoSourcemapNode(
						workspacePath,
						projectNodeInner,
						sourcemapNodeInner
					);
				}
			}
		}
	}
};
