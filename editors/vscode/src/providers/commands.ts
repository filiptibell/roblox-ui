import * as vscode from "vscode";

import { reloadAllWorkspaces } from "../workspaces";

import { MetadataProvider } from "./metadata";
import { ExplorerItem, ExplorerTreeProvider } from "../explorer";

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
		treeDataProvider: ExplorerTreeProvider
	) {
		this.register("explorer.refresh", reloadAllWorkspaces);

		this.register("explorer.focus", (item: ExplorerItem) => {
			treeView.reveal(item, {
				expand: false,
				select: false,
				focus: true,
			});
		});

		this.register("explorer.openRojoManifest", (item: ExplorerItem) => {
			const filePath = item.domInstance.metadata?.paths.rojo;
			if (filePath) {
				vscode.commands.executeCommand(
					"vscode.open",
					vscode.Uri.file(filePath)
				);
			}
		});
		this.register("explorer.openWallyManifest", (item: ExplorerItem) => {
			const filePath = item.domInstance.metadata?.paths.wally;
			if (filePath) {
				vscode.commands.executeCommand(
					"vscode.open",
					vscode.Uri.file(filePath)
				);
			}
		});

		const createInstance = async (
			item: ExplorerItem,
			classNameOrInsertService: string | boolean | void
		) => {
			// TODO: Re-implement this
			// const [created, creationResult] = await promptNewInstanceCreation(
			// 	treeDataProvider.settingsProvider,
			// 	treeDataProvider.metadataProvider,
			// 	treeDataProvider.iconsProvider,
			// 	item.getFolderPath(),
			// 	item.getFilePath(),
			// 	classNameOrInsertService
			// );
			// if (created && creationResult) {
			// 	// Open the new file path in the editor (if any)
			// 	const filePath = findPrimaryFilePath(creationResult);
			// 	if (filePath) {
			// 		vscode.commands.executeCommand(
			// 			"vscode.open",
			// 			vscode.Uri.file(filePath)
			// 		);
			// 	}
			// }
		};

		this.register("explorer.insertObject", (item: ExplorerItem) => {
			createInstance(item);
		});
		this.register("explorer.insertFolder", (item: ExplorerItem) => {
			createInstance(item, "Folder");
		});
		this.register("explorer.insertService", (item: ExplorerItem) => {
			createInstance(item, true);
		});

		this.register("explorer.renameObject", async (item: ExplorerItem) => {
			// TODO: Re-implement this
			// await promptRenameExistingInstance(
			// 	item.getFolderPath(),
			// 	item.getFilePath()
			// );
		});
		this.register("explorer.deleteObject", async (item: ExplorerItem) => {
			// TODO: Re-implement this
			// await deleteExistingInstance(
			// 	item.getFolderPath(),
			// 	item.getFilePath()
			// );
		});
	}
}
