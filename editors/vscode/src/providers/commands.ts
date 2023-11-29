import * as vscode from "vscode";

import { reconnectAllWorkspaces } from "../workspaces";
import { ExplorerItem } from "./explorer";
import { Providers } from ".";

const EXTENSION_NAME = "roblox-ui";

export class CommandsProvider implements vscode.Disposable {
	// biome-ignore lint/suspicious/noExplicitAny:
	private readonly commands: Map<string, (...args: any[]) => any> = new Map();
	private readonly disposables: Array<vscode.Disposable> = new Array();

	constructor(public readonly providers: Providers) {
		this.register("explorer.refresh", reconnectAllWorkspaces);
		this.register("explorer.quickOpen", () => providers.quickOpen.show());

		this.register("explorer.select", async (workspacePath: string, domId: string) => {
			const item = providers.explorerTree.findById(workspacePath, domId);
			if (item) {
				await providers.explorerView.reveal(item, {
					expand: false,
					select: true,
					focus: false,
				});
			}
		});
		this.register(
			"explorer.expand",
			async (workspacePath: string, domId: string, levels?: number | null) => {
				const item = providers.explorerTree.findById(workspacePath, domId);
				if (item) {
					await providers.explorerView.reveal(item, {
						expand: levels ?? true,
						select: false,
						focus: false,
					});
				}
			},
		);

		const revealFileInOS = (item: ExplorerItem) => {
			const uri = item.resourceUri;
			if (uri) {
				vscode.commands.executeCommand("revealFileInOS", uri);
			}
		};
		this.register("explorer.revealFileInOS.windows", revealFileInOS);
		this.register("explorer.revealFileInOS.mac", revealFileInOS);
		this.register("explorer.revealFileInOS", revealFileInOS);

		this.register("explorer.openRojoManifest", (item: ExplorerItem) => {
			const filePath = item.domInstance.metadata?.paths?.rojo;
			if (filePath) {
				vscode.commands.executeCommand("vscode.open", vscode.Uri.file(filePath));
			}
		});
		this.register("explorer.openWallyManifest", (item: ExplorerItem) => {
			const filePath = item.domInstance.metadata?.paths?.wally;
			if (filePath) {
				vscode.commands.executeCommand("vscode.open", vscode.Uri.file(filePath));
			}
		});

		this.register("explorer.insertObject", (item: ExplorerItem) => {
			providers.insertInstance.show(item.workspacePath, item.domInstance, null, false);
		});
		this.register("explorer.insertFolder", (item: ExplorerItem) => {
			providers.insertInstance.show(item.workspacePath, item.domInstance, "Folder", false);
		});
		this.register("explorer.insertService", (item: ExplorerItem) => {
			providers.insertInstance.show(item.workspacePath, item.domInstance, null, true);
		});

		this.register("explorer.renameObject", async (item: ExplorerItem) => {
			providers.renameInstance.show(item.workspacePath, item.domInstance);
		});
		this.register("explorer.deleteObject", async (item: ExplorerItem) => {
			await providers.explorerTree.deleteInstance(item.workspacePath, item.domInstance.id);
		});
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
		this.commands.clear();
	}

	// biome-ignore lint/suspicious/noExplicitAny:
	private register(name: string, command: (...args: any[]) => any) {
		const fullName = `${EXTENSION_NAME}.${name}`;
		const disposable = vscode.commands.registerCommand(fullName, command);
		this.disposables.push(disposable);
	}

	// biome-ignore lint/suspicious/noExplicitAny:
	public async run(name: string, ...args: any) {
		const fullName = `${EXTENSION_NAME}.${name}`;
		await vscode.commands.executeCommand(fullName, ...args);
	}
}
