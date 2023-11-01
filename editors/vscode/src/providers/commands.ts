import * as vscode from "vscode";

import {
	deleteExistingInstance,
	promptNewInstanceCreation,
	promptRenameExistingInstance,
} from "../utils/instances";

import { reloadAllWorkspaces } from "../workspaces";

import { RojoTreeItem, RojoTreeProvider } from "./explorer";

import { findPrimaryFilePath } from "../utils/sourcemap";
import { MetadataProvider } from "./metadata";

export class CommandsProvider implements vscode.Disposable {
	private commands: Map<string, (...args: any[]) => any> = new Map();
	private disposables: Array<vscode.Disposable> = new Array();

	private register(name: string, command: (...args: any[]) => any) {
		const fullName = `roblox-ui.${name}`;
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
		metadata: MetadataProvider,
		treeView: vscode.TreeView<vscode.TreeItem>,
		treeDataProvider: RojoTreeProvider
	) {
		this.register("refresh", reloadAllWorkspaces);
		this.register("openProjectFile", (item: RojoTreeItem) => {
			item.openFile();
		});
		/*
			The below commands modify tree items and are somewhat special:

			The sourcemap might need a bit of time to regenerate after
			an item is changed, but since we know exactly what got changed
			and have direct access to the tree item that was changed and
			its parent we can manually change it in the sourcemap too
			and update the item to give the user instant feedback
		*/
		const createInstance = async (
			item: RojoTreeItem,
			classNameOrInsertService: string | boolean | void
		) => {
			const [created, creationResult] = await promptNewInstanceCreation(
				treeDataProvider.settingsProvider,
				treeDataProvider.metadataProvider,
				treeDataProvider.iconsProvider,
				item.getFolderPath(),
				item.getFilePath(),
				classNameOrInsertService
			);
			if (created && creationResult) {
				// Open the new file path in the editor (if any)
				const filePath = findPrimaryFilePath(creationResult);
				if (filePath) {
					vscode.commands.executeCommand(
						"vscode.open",
						vscode.Uri.file(filePath)
					);
				}
			}
		};
		this.register("insertObject", (item: RojoTreeItem) => {
			createInstance(item);
		});
		this.register("insertFolder", (item: RojoTreeItem) => {
			createInstance(item, "Folder");
		});
		this.register("insertService", (item: RojoTreeItem) => {
			createInstance(item, true);
		});
		this.register("renameObject", async (item: RojoTreeItem) => {
			await promptRenameExistingInstance(
				item.getFolderPath(),
				item.getFilePath()
			);
		});
		this.register("deleteObject", async (item: RojoTreeItem) => {
			await deleteExistingInstance(
				item.getFolderPath(),
				item.getFilePath()
			);
		});
	}
}
