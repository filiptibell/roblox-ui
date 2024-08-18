import * as vscode from "vscode"

import { SettingsProvider } from "./settings"
import { MetadataProvider } from "./metadata"
import { IconsProvider } from "./icons"
import { CommandsProvider } from "./commands"

import { ExplorerItem, ExplorerTreeProvider } from "./explorer"
import { RenameInstanceProvider } from "./renameInstance"
import { QuickOpenProvider } from "./quickOpen"
import { SelectionProvider } from "./selection"
import { InsertInstanceProvider } from "./insertInstance"

export class Providers implements vscode.Disposable {
	private readonly disposables: vscode.Disposable[]

	public readonly settings: SettingsProvider
	public readonly metadata: MetadataProvider
	public readonly icons: IconsProvider
	public readonly commands: CommandsProvider

	public readonly explorerTree: ExplorerTreeProvider
	public readonly explorerView: vscode.TreeView<ExplorerItem>

	public readonly insertInstance: InsertInstanceProvider
	public readonly renameInstance: RenameInstanceProvider
	public readonly quickOpen: QuickOpenProvider
	public readonly selection: SelectionProvider

	constructor(public readonly extensionContext: vscode.ExtensionContext) {
		// Basic providers used by all others
		this.settings = new SettingsProvider(this)
		this.metadata = new MetadataProvider(this)
		this.icons = new IconsProvider(this)
		this.commands = new CommandsProvider(this)

		// Main tree view and data providers
		this.explorerTree = new ExplorerTreeProvider(this)
		this.explorerView = vscode.window.createTreeView("roblox-ui.explorer", {
			treeDataProvider: this.explorerTree,
			showCollapseAll: true,
			canSelectMany: false,
		})

		// Providers that depend on above
		this.insertInstance = new InsertInstanceProvider(this)
		this.renameInstance = new RenameInstanceProvider(this)
		this.quickOpen = new QuickOpenProvider(this)
		this.selection = new SelectionProvider(this)

		// Store them all to dispose properly later
		this.disposables = [
			this.settings,
			this.metadata,
			this.icons,
			this.commands,
			this.explorerTree,
			this.explorerView,
			this.insertInstance,
			this.renameInstance,
			this.quickOpen,
			this.selection,
		]
	}

	public dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose()
		}
	}
}
