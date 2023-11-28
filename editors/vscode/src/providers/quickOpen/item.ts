import * as vscode from "vscode";

import { DomInstance } from "../../server";
import { Providers } from "..";

export class QuickOpenItem implements vscode.QuickPickItem {
	public readonly alwaysShow: boolean = true;
	public readonly label: string;
	public readonly iconPath?: { light: vscode.Uri; dark: vscode.Uri };
	public readonly description?: string;

	constructor(
		public readonly providers: Providers,
		public readonly workspacePath: string,
		public readonly domInstance: DomInstance,
		readonly fullName: string[] | null,
	) {
		this.label = domInstance.name;

		this.iconPath = providers.icons.getClassIcon(
			providers.settings.get("explorer.iconPack"),
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
		await this.providers.explorerTree.revealById(
			this.workspacePath,
			this.domInstance.id,
			select,
		);
	}
}
