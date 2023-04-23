import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs/promises";

import { SettingsManager } from "./settings";
import { RojoTreeProvider } from "../provider";
import { rojoSourcemapWatch } from "./rojo";

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
