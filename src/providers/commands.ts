import * as vscode from "vscode";

import {
	deleteExistingInstance,
	promptNewInstanceCreation,
	promptRenameExistingInstance,
} from "../utils/instances";

import { reloadAllWorkspaces } from "../workspaces";

import { RojoTreeItem, RojoTreeProvider } from "./explorer";
import { IconsProvider } from "./icons";

import { clearRobloxCache } from "../web/roblox";
import {
	SourcemapNode,
	cloneSourcemapNode,
	findPrimaryFilePath,
} from "../utils/sourcemap";

export class CommandsProvider implements vscode.Disposable {
	private commands: Map<string, (...args: any[]) => any> = new Map();
	private disposables: Array<vscode.Disposable> = new Array();

	private register(name: string, command: (...args: any[]) => any) {
		const fullName = `rojoViewer.${name}`;
		const disposable = vscode.commands.registerCommand(fullName, command);
		this.disposables.push(disposable);
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
		this.commands.clear();
		this.commands = new Map();
		this.disposables = new Array();
	}

	constructor(
		context: vscode.ExtensionContext,
		treeView: vscode.TreeView<vscode.TreeItem>,
		treeDataProvider: RojoTreeProvider,
		iconsProvider: IconsProvider
	) {
		this.register("refresh", reloadAllWorkspaces);
		this.register("openProjectFile", (item: RojoTreeItem) => {
			item.openFile();
		});
		this.register("clearCache", async () => {
			try {
				await iconsProvider.clearCachedIcons();
				clearRobloxCache(context, true);
			} catch (err) {
				vscode.window.showWarningMessage(
					`Failed to clear cache!\n\n${err}`
				);
			}
		});
		/*
			The below commands modify tree items and are somewhat special:

			The sourcemap might need a bit of time to regenerate after
			an item is changed, but since we know exactly what got changed
			and have direct access to the tree item that was changed and
			its parent we can manually change it in the sourcemap too
			and update the item to give the user instant feedback
		*/
		const createInstance = async (item: RojoTreeItem, service: boolean) => {
			const [created, creationResult] = await promptNewInstanceCreation(
				item.getFolderPath(),
				item.getFilePath(),
				service
			);
			if (created && creationResult) {
				const node = item.getNode() ?? undefined;
				if (node) {
					// Create a new sourcemap node to insert as a child
					const createdNode: SourcemapNode = { ...creationResult };
					const newNode = cloneSourcemapNode(node);
					if (newNode.children) {
						newNode.children.push(createdNode);
					} else {
						newNode.children = [createdNode];
					}

					// Update the parent
					await item.update(newNode);

					// Reveal the new child in the explorer
					const children = await item.getChildren();
					const child = children.find(
						(child) => child.getNode() === createdNode
					);
					if (child) {
						treeView.reveal(child);
					}

					// Open the new file path in the editor (if any)
					const filePath = findPrimaryFilePath(createdNode);
					if (filePath) {
						vscode.commands.executeCommand(
							"vscode.open",
							vscode.Uri.file(filePath)
						);
					}
				}
			}
		};
		this.register("insertObject", (item: RojoTreeItem) => {
			createInstance(item, false);
		});
		this.register("insertService", (item: RojoTreeItem) => {
			createInstance(item, true);
		});
		this.register("renameObject", async (item: RojoTreeItem) => {
			const [renamed, renameResult] = await promptRenameExistingInstance(
				item.getFolderPath(),
				item.getFilePath()
			);
			if (renamed && renameResult) {
				const node = item.getNode() ?? undefined;
				const parent = item.getParent() ?? undefined;
				const parentNode = parent ? parent.getNode() : null;
				if (node && parent && parentNode) {
					const newNode = cloneSourcemapNode(node);
					if (renameResult.name) {
						newNode.name = renameResult.name;
					}
					if (renameResult.folderPath) {
						newNode.folderPath = renameResult.folderPath;
					} else if (renameResult.filePaths) {
						newNode.filePaths = renameResult.filePaths;
					}
					item.update(newNode);
				}
			}
		});
		this.register("deleteObject", async (item: RojoTreeItem) => {
			const deleted = await deleteExistingInstance(
				item.getFolderPath(),
				item.getFilePath()
			);
			if (deleted) {
				const node = item.getNode() ?? undefined;
				const parent = item.getParent() ?? undefined;
				const parentNode = parent ? parent.getNode() : null;
				if (node && parent && parentNode) {
					const newParentNode = cloneSourcemapNode(parentNode);
					const oldNodeIndex = newParentNode.children?.findIndex(
						(child) => child === node
					);
					if (oldNodeIndex !== undefined && oldNodeIndex !== -1) {
						if (newParentNode.children!.length === 1) {
							newParentNode.children = undefined;
						} else {
							newParentNode.children!.splice(oldNodeIndex, 1);
						}
						parent.update(newParentNode);
					}
				}
			}
		});
	}
}
