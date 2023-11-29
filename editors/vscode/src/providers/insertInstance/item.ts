import * as vscode from "vscode";

import { Providers } from "..";

export class InsertInstanceItem implements vscode.QuickPickItem {
	public readonly label: string;
	public readonly iconPath?: { light: vscode.Uri; dark: vscode.Uri };
	public readonly detail?: string;
	public readonly kind?: vscode.QuickPickItemKind = vscode.QuickPickItemKind.Default;

	constructor(
		public readonly providers: Providers,
		public readonly workspacePath: string,
		public readonly className: string,
	) {
		this.label = className;
		this.iconPath = providers.icons.getClassIcon(
			providers.settings.get("explorer.iconPack"),
			className,
		);
	}
}

export class InsertInstanceSeparator implements vscode.QuickPickItem {
	public readonly label: string = "";
	public readonly kind?: vscode.QuickPickItemKind = vscode.QuickPickItemKind.Separator;

	constructor(
		public readonly providers: Providers,
		public readonly workspacePath: string,
		public readonly category?: string,
	) {
		if (category && category.length > 0) {
			this.label = category;
		}
	}
}
