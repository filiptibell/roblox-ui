import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs/promises";
import * as cp from "child_process";

import { SettingsProvider } from "../providers/settings";
import { RojoTreeProvider } from "../providers/tree";

import { RobloxReflectionMetadata } from "../web/robloxReflectionMetadata";

import {
	ProjectRootNode,
	isInitFilePath,
	mergeProjectIntoSourcemap,
	cacheProjectFileSystemPaths,
	rojoSourcemapWatch,
	isBinaryFilePath,
} from "./rojo";

export type SourcemapNode = {
	name: string;
	className: string;
	folderPath?: string;
	filePaths?: string[];
	children?: SourcemapNode[];
};

const postprocessSourcemap = (
	node: SourcemapNode,
	parent: SourcemapNode | void
): SourcemapNode => {
	// Init files have a guaranteed parent directory
	if (!node.folderPath && node.filePaths) {
		for (const filePath of node.filePaths.values()) {
			if (isInitFilePath(filePath)) {
				node.folderPath = path.dirname(filePath);
			}
		}
	}
	// Otherwise we look at the parent of this node and try to
	// join its folder path with this sourcemap node folder name
	if (!node.folderPath && node.className === "Folder") {
		if (parent && parent.folderPath) {
			node.folderPath = path.join(parent.folderPath, node.name);
		}
	}
	if (node.children) {
		for (const child of node.children.values()) {
			postprocessSourcemap(child, node);
		}
	}
	return node;
};

export const findPrimaryFilePath = (
	workspacePath: string,
	node: SourcemapNode,
	allowBinaryFiles: boolean | void
): string | null => {
	if (node.filePaths) {
		if (node.filePaths.length === 1) {
			if (allowBinaryFiles || !isBinaryFilePath(node.filePaths[0])) {
				return path.join(workspacePath, node.filePaths[0]);
			}
		} else {
			// TODO: Sort and find using ordering - init, lua, model, meta, project
			const copied = node.filePaths.slice();
		}
	}
	return null;
};

export const getSourcemapNodeTreeOrder = (
	node: SourcemapNode,
	reflectionMetadata: RobloxReflectionMetadata | undefined | null | void
): number => {
	if (reflectionMetadata) {
		const metadata = reflectionMetadata.Classes.get(node.className);
		if (metadata && metadata.ExplorerOrder) {
			return metadata.ExplorerOrder;
		}
	}
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

export const connectSourcemapUsingRojo = (
	workspacePath: string,
	settings: SettingsProvider,
	treeProvider: RojoTreeProvider
): [Function, Function] => {
	let destroyed: boolean = false;
	let projectFileContents: string | null = null;
	let currentChildProcess: cp.ChildProcessWithoutNullStreams | null = null;

	// Set as loading right away to let the user know
	treeProvider.setLoading(workspacePath);

	// Create a file watcher for the project file
	const projectFilePath = `${workspacePath}/${
		settings.get("rojoProjectFile") || "default.project.json"
	}`;
	const projectFileWatcher =
		vscode.workspace.createFileSystemWatcher(projectFilePath);

	// Create the callback that will watch
	// and generate a sourcemap for our tree
	const updateProjectFile = async (contents: string | null) => {
		if (projectFileContents !== contents) {
			projectFileContents = contents;
			// Kill any previous child process
			if (currentChildProcess) {
				currentChildProcess.kill();
				currentChildProcess = null;
			}
			// If we got a project file, spawn a new rojo process
			// that will generate sourcemaps and watch for changes
			if (projectFileContents && !destroyed) {
				// Parse the project file and get rid of file paths (not directories)
				let projectFileNode: ProjectRootNode | undefined;
				try {
					projectFileNode = JSON.parse(projectFileContents);
					if (projectFileNode) {
						await cacheProjectFileSystemPaths(
							workspacePath,
							projectFileNode
						);
					}
				} catch (e) {
					vscode.window.showWarningMessage(
						`Rojo Explorer failed to read the project file at ${projectFilePath}` +
							"\nSome explorer functionality may not be available" +
							`\n${e}`
					);
					return;
				}
				if (destroyed) {
					return;
				}
				// Spawn the rojo process
				currentChildProcess = rojoSourcemapWatch(
					workspacePath,
					settings,
					() => {
						treeProvider.setLoading(workspacePath);
					},
					(_, sourcemap) => {
						if (projectFileNode) {
							mergeProjectIntoSourcemap(
								workspacePath,
								projectFileNode,
								sourcemap
							);
						}
						postprocessSourcemap(sourcemap);
						treeProvider.update(workspacePath, sourcemap);
					}
				);
			}
		}
	};

	// Create callback for manually updating the sourcemap
	const update = () => {
		if (destroyed) {
			return;
		}
		rojoSourcemapWatch(
			workspacePath,
			settings,
			() => {
				treeProvider.setLoading(workspacePath);
			},
			(childProcess, sourcemap) => {
				treeProvider.update(workspacePath, sourcemap);
				childProcess.kill();
			}
		);
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		if (!destroyed) {
			destroyed = true;
			updateProjectFile(null);
			projectFileWatcher.dispose();
			treeProvider.delete(workspacePath);
		}
	};

	// Listen to the project file changing and read it once initially
	const readProjectFile = async () => {
		fs.readFile(projectFilePath, "utf8")
			.then(updateProjectFile)
			.catch((e) => {
				vscode.window.showErrorMessage(
					`Rojo Explorer failed to read the project file at ${projectFilePath}\n${e}`
				);
			});
	};
	projectFileWatcher.onDidChange(readProjectFile);
	readProjectFile();

	return [update, destroy];
};

export const connectSourcemapUsingFile = (
	workspacePath: string,
	settings: SettingsProvider,
	treeProvider: RojoTreeProvider
): [Function, Function] => {
	// Create a file watcher for the sourcemap
	const sourcemapPath = `${workspacePath}/sourcemap.json`;
	const fileWatcher = vscode.workspace.createFileSystemWatcher(sourcemapPath);

	// Create callback for updating sourcemap
	const update = () => {
		fs.readFile(sourcemapPath, "utf8")
			.then(JSON.parse)
			.then((sourcemap: SourcemapNode) => {
				postprocessSourcemap(sourcemap);
				treeProvider.update(workspacePath, sourcemap);
			});
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		treeProvider.delete(workspacePath);
		fileWatcher.dispose();
	};

	// Start watching the sourcemap for changes and update once initially
	fileWatcher.onDidChange(update);
	update();

	return [update, destroy];
};
