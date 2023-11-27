import * as vscode from "vscode";

import { SettingsProvider } from "../settings";
import { MetadataProvider } from "../metadata";
import { IconsProvider } from "../icons";

import { DomInstance } from "../../server";

export class QuickOpenItem implements vscode.QuickPickItem {
	public readonly alwaysShow: boolean = true;
	public readonly label: string;
	public readonly iconPath?: { light: vscode.Uri; dark: vscode.Uri };
	public readonly description?: string;

	constructor(
		public readonly settingsProvider: SettingsProvider,
		public readonly metadataProvider: MetadataProvider,
		public readonly iconsProvider: IconsProvider,
		public readonly workspacePath: string,
		public readonly domInstance: DomInstance,
		readonly fullName: string[] | null,
	) {
		this.label = domInstance.name;

		this.iconPath = iconsProvider.getClassIcon(
			settingsProvider.get("explorer.iconPack"),
			domInstance.className,
		);

		if (fullName) {
			const wfolders = vscode.workspace.workspaceFolders;
			if (!wfolders || wfolders.length <= 1) {
				fullName.shift(); // Don't show root name for single-root workspaces
			}
			fullName.pop(); // Don't show name, its included in the label
			this.description = fullName.join(".");
		}
	}

	public async open() {
		const canOpen = this.domInstance.metadata?.actions?.canOpen;
		const filePath = this.domInstance.metadata?.paths?.file;
		if (canOpen && filePath) {
			await vscode.commands.executeCommand("vscode.open", vscode.Uri.file(filePath));
			return true;
		}
		return false;
	}

	public async reveal(select?: true | null) {
		await vscode.commands.executeCommand(
			"roblox-ui.explorer.reveal",
			this.workspacePath,
			this.domInstance.id,
			select,
		);
	}
}
