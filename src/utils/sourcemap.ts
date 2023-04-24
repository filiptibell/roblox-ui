import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs/promises";
import * as cp from "child_process";

import { SettingsProvider } from "../providers/settings";
import { RojoTreeProvider } from "../providers/tree";
import {
	ProjectOrMetaFileNode,
	ProjectRootNode,
	extractRojoFileExtension,
	isInitFilePath,
	isProjectFilePath,
	rojoSourcemapWatch,
} from "./rojo";
import { RobloxReflectionMetadata } from "../web/robloxReflectionMetadata";

export type SourcemapNode = {
	name: string;
	className: string;
	folderPath?: string;
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
	// Some instance may have a direct folder path tied
	// to them, depending on how they were created
	if (node.folderPath) {
		return path.join(workspaceRoot, node.folderPath);
	}
	// Other folders are trickier, and the sourcemap does
	// not contain information for them, so we need to try
	// and use our root folders parsed from the project file
	if (node.className === "Folder") {
		const parts = [];
		let current = node;
		while (current && !current.folderPath) {
			parts.push(current.name);
			if (current.parent) {
				current = current.parent;
			} else {
				break;
			}
		}
		if (current.folderPath) {
			parts.reverse();
			return path.join(workspaceRoot, current.folderPath, ...parts);
		}
	}
	return null;
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
				const projectFileNode: ProjectRootNode =
					JSON.parse(projectFileContents);
				currentChildProcess = rojoSourcemapWatch(
					workspacePath,
					settings,
					() => {
						treeProvider.setLoading(workspacePath);
					},
					async (_, sourcemap) => {
						try {
							await mergeProjectIntoSourcemap(
								workspacePath,
								projectFileNode,
								sourcemap
							);
						} catch (e) {
							vscode.window.showWarningMessage(
								`Rojo Explorer partially failed to read the project file at ${projectFilePath}` +
									"\nSome explorer functionality may not be available" +
									`\n${e}`
							);
							return;
						}
						if (destroyed) {
							return;
						}
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
			.then(parseSourcemap)
			.then((sourcemap) => {
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

export const mergeProjectIntoSourcemap = async (
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
	await mergeProjectNodeIntoSourcemapNode(
		workspacePath,
		rootAsNode,
		sourcemapAsRoot
	);
};

export const mergeProjectNodeIntoSourcemapNode = async (
	workspacePath: string,
	projectNode: ProjectOrMetaFileNode,
	sourcemapNode: SourcemapNode
): Promise<void> => {
	const nodePath = projectNode["$path"];
	if (nodePath) {
		const fullPath = path.join(workspacePath, nodePath);
		try {
			if ((await fs.stat(fullPath)).isDirectory()) {
				sourcemapNode.folderPath = nodePath;
			}
		} catch {}
	}
	const innerPromises: Promise<void>[] = [];
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
					innerPromises.push(
						mergeProjectNodeIntoSourcemapNode(
							workspacePath,
							projectNodeInner,
							sourcemapNodeInner
						)
					);
				}
			}
		}
	}
	if (innerPromises.length > 0) {
		await Promise.all(innerPromises);
	}
};
