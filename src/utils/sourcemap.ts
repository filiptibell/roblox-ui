import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs/promises";

import { SettingsManager } from "./settings";
import { RojoTreeProvider } from "../provider";
import {
	extractRojoFileExtension,
	isInitFilePath,
	isProjectFilePath,
	rojoSourcemapWatch,
} from "./rojo";

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
): string | null => {
	if (node.filePaths) {
		const filePath = node.filePaths.find((filePath) => {
			return (
				isProjectFilePath(filePath) ||
				extractRojoFileExtension(filePath) !== null
			);
		});
		if (filePath) {
			return path.join(workspaceRoot, filePath);
		}
	}
	return null;
};

export const findFolderPath = (
	workspaceRoot: string,
	node: SourcemapNode
): string | null => {
	// Init files are easy, they have a guaranteed parent folder
	const filePath = findFilePath(workspaceRoot, node);
	if (filePath) {
		if (isProjectFilePath(filePath)) {
			return null;
		} else if (isInitFilePath(filePath)) {
			return path.join(workspaceRoot, path.dirname(filePath));
		}
	}
	// Other folders are trickier, and the sourcemap does not contain information for them
	if (node.className === "Folder") {
		// Look for the first descendant that has a known file path
		let foundFilePath: string | undefined;
		let currentDepth = 0;
		let currentNode: SourcemapNode | undefined = node;
		while (currentNode) {
			if (currentNode.children && currentNode.children.length > 0) {
				currentNode = currentNode.children[0];
				currentDepth += 1;
			} else {
				break;
			}
			const filePath = currentNode
				? findFilePath(workspaceRoot, currentNode)
				: null;
			if (filePath) {
				foundFilePath = filePath;
				break;
			}
		}
		if (foundFilePath && currentDepth > 0) {
			// We found a file and we know how deep in the hierarchy it is,
			// so we step up the number of directories deep it was found
			const parts = [path.dirname(foundFilePath)];
			if (currentDepth > 0) {
				for (let index = 1; index < currentDepth; index++) {
					parts.push("..");
				}
			}
			const folderPath = path.resolve(...parts);
			return folderPath;
		}
	}
	return null;
};

export const connectSourcemapUsingRojo = (
	workspacePath: string,
	settings: SettingsManager,
	treeDataProvider: RojoTreeProvider
): [Function, Function] => {
	// Spawn a new rojo process that will generate sourcemaps and watch for changes
	const childProcess = rojoSourcemapWatch(
		workspacePath,
		settings,
		() => {
			treeDataProvider.setLoading(workspacePath);
		},
		(_, sourcemap) => {
			treeDataProvider.update(workspacePath, sourcemap);
		}
	);

	// Create callback for manually updating the sourcemap
	const update = () => {
		rojoSourcemapWatch(
			workspacePath,
			settings,
			() => {
				treeDataProvider.setLoading(workspacePath);
			},
			(childProcess, sourcemap) => {
				treeDataProvider.update(workspacePath, sourcemap);
				childProcess.kill();
			}
		);
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		treeDataProvider.delete(workspacePath);
		childProcess.kill();
	};

	return [update, destroy];
};

export const connectSourcemapUsingFile = (
	workspacePath: string,
	settings: SettingsManager,
	treeDataProvider: RojoTreeProvider
): [Function, Function] => {
	// Create a file watcher for the sourcemap
	const sourcemapPath = `${workspacePath}/sourcemap.json`;
	const fileWatcher = vscode.workspace.createFileSystemWatcher(sourcemapPath);

	// Create callback for updating sourcemap
	const update = () => {
		fs.readFile(sourcemapPath, "utf8").then((sourcemapJson) => {
			treeDataProvider.update(
				workspacePath,
				parseSourcemap(sourcemapJson)
			);
		});
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		treeDataProvider.delete(workspacePath);
		fileWatcher.dispose();
	};

	// Start watching the sourcemap for changes and update once initially
	fileWatcher.onDidChange(update);
	update();

	return [update, destroy];
};
