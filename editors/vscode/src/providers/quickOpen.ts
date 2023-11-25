import * as vscode from "vscode";
import * as path from "path";

import { SettingsProvider } from "./settings";
import { MetadataProvider } from "./metadata";
import { IconsProvider } from "./icons";
import { ExplorerTreeProvider } from "../explorer";

import { DomInstance } from "../server";

const MINIMUM_QUERY_LENGTH = 1;

export class QuickOpenProvider implements vscode.Disposable {
	private readonly picker: vscode.QuickPick<QuickOpenItem>;
	private readonly disposables: vscode.Disposable[] = [];

	constructor(
		private readonly settingsProvider: SettingsProvider,
		private readonly metadataProvider: MetadataProvider,
		private readonly iconsProvider: IconsProvider,
		private readonly explorerProvider: ExplorerTreeProvider
	) {
		this.picker = vscode.window.createQuickPick();
		this.picker.canSelectMany = false;
		this.picker.placeholder = "Search...";
		this.picker.title = "Quick Open";

		this.picker.matchOnDescription = true;
		(this.picker as any).sortByLabel = false;

		const onChange = () => this.update();
		const onAccept = () => this.accept();
		const onHide = () => this.hide();

		this.disposables.push(this.picker.onDidChangeValue(onChange));
		this.disposables.push(this.picker.onDidAccept(onAccept));
		this.disposables.push(this.picker.onDidHide(onHide));
	}

	dispose() {
		this.picker.dispose();
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}

	private async update() {
		const query = this.picker.value;
		if (query.length <= MINIMUM_QUERY_LENGTH) {
			this.picker.items = [];
			return;
		}

		this.picker.busy = true;

		const searchResponsePromises = new Array<
			Promise<[string, DomInstance[]]>
		>();
		for (const workspacePath of this.explorerProvider.getWorkspacePaths()) {
			searchResponsePromises.push(
				new Promise((resolve, reject) => {
					this.explorerProvider
						.findByQuery(workspacePath, query)
						.then((instances) =>
							resolve([workspacePath, instances])
						)
						.catch(reject);
				})
			);
		}
		const searchResponses = await Promise.all(searchResponsePromises);

		this.picker.busy = false;

		const newNames = new Array<string>();
		const newItems = new Array<QuickOpenItem>();
		for (const [workspacePath, foundInstances] of searchResponses) {
			for (const foundInstance of foundInstances) {
				newNames.push(foundInstance.name);
				newItems.push(
					new QuickOpenItem(
						this.settingsProvider,
						this.metadataProvider,
						this.iconsProvider,
						workspacePath,
						foundInstance
					)
				);
			}
		}

		this.picker.items = newItems;
	}

	private async accept() {
		for (const acceptedItem of this.picker.selectedItems) {
			const canOpen = acceptedItem.domInstance.metadata?.actions?.canOpen;
			const filePath = acceptedItem.domInstance.metadata?.paths?.file;
			if (canOpen && filePath) {
				vscode.commands.executeCommand(
					"vscode.open",
					vscode.Uri.file(filePath)
				);
			}
		}
		this.hide();
	}

	public show() {
		this.picker.items = [];
		this.picker.value = "";
		this.picker.show();
		this.update();
	}

	public hide() {
		this.picker.hide();
		this.picker.value = "";
		this.picker.items = [];
	}
}

export class QuickOpenItem implements vscode.QuickPickItem {
	public alwaysShow: boolean = true;
	public label: string = "QuickOpenItem";
	public iconPath?: { light: vscode.Uri; dark: vscode.Uri };
	public detail?: string;

	constructor(
		private readonly settingsProvider: SettingsProvider,
		private readonly metadataProvider: MetadataProvider,
		private readonly iconsProvider: IconsProvider,
		public readonly workspacePath: string,
		public readonly domInstance: DomInstance
	) {
		this.label = domInstance.name;

		this.iconPath = iconsProvider.getClassIcon(
			settingsProvider.get("explorer.iconPack"),
			domInstance.className
		);

		const filePath = domInstance.metadata?.paths?.file;
		if (filePath) {
			// TODO: Use instance full path instead of file path
			const relativePath = filePath.slice(workspacePath.length + 1);
			const parsedDirs = path.parse(relativePath).dir.split(path.sep);
			this.detail = parsedDirs.join(" → ");
		}
	}
}
