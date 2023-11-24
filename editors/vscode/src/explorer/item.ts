import * as vscode from "vscode";

import { DomInstance } from "../server";

import { ExplorerTreeProvider } from ".";

export class ExplorerItem extends vscode.TreeItem {
	private childReferences: ExplorerItem[] = [];

	constructor(
		public readonly parent: ExplorerItem | null,
		public readonly workspacePath: string,
		public readonly treeProvider: ExplorerTreeProvider,
		public readonly domInstance: DomInstance,
		public readonly isRoot: boolean
	) {
		super(domInstance.name);

		// Set new resource uri for git / diagnostics decorations
		const filePath = domInstance.metadata?.paths.file;
		const folderPath = domInstance.metadata?.paths.folder;
		const resourceUri = filePath
			? vscode.Uri.file(filePath)
			: folderPath
			? vscode.Uri.file(folderPath)
			: isRoot
			? vscode.Uri.file(workspacePath)
			: undefined;
		this.resourceUri = resourceUri;

		// Set collapsible state to show expansion arrow for children, or not
		const [collapsibleState, shouldSelect] = getInitialTreeItemState(
			workspacePath,
			domInstance,
			isRoot
		);
		this.collapsibleState = collapsibleState;

		// Set name, description, tooltip, icon
		this.label = domInstance.name;
		this.description = getInstanceDescription(
			workspacePath,
			treeProvider,
			domInstance,
			resourceUri
		);
		this.tooltip = getInstanceTooltip(treeProvider, domInstance);
		this.iconPath = treeProvider.iconsProvider.getClassIcon(
			treeProvider.settingsProvider.get("explorer.iconPack"),
			domInstance.className
		);

		// If this instance can be clicked to open, link that up
		if (domInstance.metadata?.actions.canOpen) {
			this.command = {
				title: "Open file",
				command: "vscode.open",
				arguments: [resourceUri],
			};
		}

		// Set context value for menu actions such as copy,
		// paste, insert object, rename, ... to appear correctly
		this.contextValue = getInstanceContextValue(
			treeProvider,
			domInstance,
			isRoot
		);

		// Finally, select the tree item if wanted
		if (shouldSelect) {
			vscode.commands.executeCommand(
				"roblox-ui.explorer.select",
				this.workspacePath,
				this.domInstance.id
			);
		}
	}

	public getServer() {
		const server = this.treeProvider.getServer(this.workspacePath);
		if (server !== undefined) {
			return server;
		} else {
			throw new Error("Missing server for explorer tree item");
		}
	}

	public setChildReferences(children: ExplorerItem[]) {
		this.childReferences = children;
	}

	public expandRevealPath(fsPath: string): ExplorerItem | null {
		if (
			fsPath === this.domInstance.metadata?.paths.file ||
			fsPath === this.domInstance.metadata?.paths.folder
		) {
			return this;
		}

		let found: ExplorerItem | null = null;

		// Check for an exact child match
		for (const child of this.childReferences) {
			if (
				fsPath === child.domInstance.metadata?.paths.file ||
				fsPath === child.domInstance.metadata?.paths.folder
			) {
				found = child;
				break;
			}
		}

		// Check for folder partial match if there was no exact child match
		if (found === null) {
			for (const child of this.childReferences) {
				const folderPath = child.domInstance.metadata?.paths.folder;
				if (folderPath && fsPath.startsWith(folderPath)) {
					found = child;
					break;
				}
			}
		}

		// If we found a child, expand this, and keep looking
		if (found !== null) {
			// FIXME: Setting state here does not seem to actually expand in explorer view ...
			this.collapsibleState = vscode.TreeItemCollapsibleState.Expanded;
			return found.expandRevealPath(fsPath);
		}

		return null;
	}
}

/**
	Gets the initial `vscode.TreeItemCollapsibleState` for an instance,
	as well as if the instance should be initially selected or not.
*/
const getInitialTreeItemState = (
	workspacePath: string,
	domInstance: DomInstance,
	isRoot: boolean
): [vscode.TreeItemCollapsibleState, boolean] => {
	const filePath = domInstance.metadata?.paths.file;

	let state = vscode.TreeItemCollapsibleState.None;
	if (domInstance.children && domInstance.children.length > 0) {
		/*
			If this is the root and we only have a single workspace
			folder, we should expand it right away, since that is
			the only action a user can and will definitely perform
		*/
		if (isRoot && vscode.workspace.workspaceFolders?.length === 1) {
			state = vscode.TreeItemCollapsibleState.Expanded;
		} else {
			state = vscode.TreeItemCollapsibleState.Collapsed;
			/*
				If any current editor is open and this instance has a
				folder that is part of the path, we should reveal it

				Doing this during creation, and as vscode calls getChildren on the tree
				chain means we end up with visible editors also revealed in the explorer
			*/
			const folderPath = domInstance.metadata?.paths.folder;
			if (folderPath) {
				for (const editor of vscode.window.visibleTextEditors) {
					const editorPath = editor.document.uri.fsPath;
					/*
						NOTE: We don't want to expand exact matches - in case we have an init
						file open that has children, we really just want the init file visible

						At this point we would also know that no other editor corresponds
						to this file which means we can skip checking those other editors
					*/
					if (filePath && editorPath.endsWith(filePath)) {
						break;
					}
					if (editorPath.startsWith(folderPath)) {
						state = vscode.TreeItemCollapsibleState.Expanded;
						break;
					}
				}
			}
		}
	}

	/*
		If this explorer item has a file path that is currently
		open and active in an editor, we should make it selected
	*/
	let shouldSelect = false;
	const editor = vscode.window.activeTextEditor;
	if (editor && filePath === editor.document.uri.fsPath) {
		shouldSelect = true;
	}

	return [state, shouldSelect];
};

/**
	Gets the inline description text for an instance.
*/
const getInstanceDescription = (
	workspacePath: string,
	treeProvider: ExplorerTreeProvider,
	domInstance: DomInstance,
	resourceUri?: vscode.Uri
) => {
	const descriptionPartials: string[] = [];

	if (treeProvider.settingsProvider.get("wally.showPackageVersion")) {
		// TODO: Add back wally version when it is part of instance metadata
		// descriptionPartials.push(domInstance.metadata.version);
	}

	if (treeProvider.settingsProvider.get("explorer.showClassNames")) {
		descriptionPartials.push(domInstance.className);
	}

	if (treeProvider.settingsProvider.get("explorer.showFilePaths")) {
		if (resourceUri) {
			const relPath = resourceUri.fsPath.slice(workspacePath.length + 1);
			descriptionPartials.push(relPath);
		}
	}

	return descriptionPartials.length > 0
		? descriptionPartials.join(" - ")
		: undefined;
};

/**
	Gets the hover tooltip markdown text for an instance.
*/
const getInstanceTooltip = (
	treeProvider: ExplorerTreeProvider,
	domInstance: DomInstance
) => {
	const classData = treeProvider.metadataProvider.getClassData(
		domInstance.className
	);

	let tooltip = "### " + domInstance.name;
	if (domInstance.className !== domInstance.name) {
		tooltip += "\n\n";
		tooltip += "#### " + domInstance.className;
	}

	let desc = classData?.description ?? null;
	if (typeof desc === "string" && desc.length > 0) {
		tooltip += "\n\n";
		tooltip += desc;
	}

	let link = classData?.documentationUrl ?? null;
	if (typeof link === "string" && link.length > 0) {
		tooltip += "\n\n";
		tooltip += "[Learn More $(link-external)](" + link + ")";
	}

	tooltip += "\n";

	let tooltipMarkdown = new vscode.MarkdownString(tooltip);
	tooltipMarkdown.supportThemeIcons = true;
	tooltipMarkdown.supportHtml = true;
	tooltipMarkdown.isTrusted = true;
	return tooltipMarkdown;
};

/**
	Gets the context value for an instance.

	The context value is a semicolon-separated string which is
	used in `enablement` and `when` in the extension manifest.
*/
const getInstanceContextValue = (
	treeProvider: ExplorerTreeProvider,
	domInstance: DomInstance,
	isRoot: boolean
) => {
	const paths = domInstance.metadata?.paths;
	const actions = domInstance.metadata?.actions;
	const classData = treeProvider.metadataProvider.getClassData(
		domInstance.className
	);

	const contextPartials = new Set();

	let hasAnyAction = false;
	if (actions) {
		for (const [key, value] of Object.entries(actions)) {
			if (value === true) {
				contextPartials.add(key);
				hasAnyAction = true;
			}
		}
	}

	if (isRoot) {
		if (paths?.rojo) {
			contextPartials.add("rojoManifest");
		}
		if (paths?.wally) {
			contextPartials.add("wallyManifest");
		}
	} else if (hasAnyAction) {
		const isService = classData?.isService ?? false;
		if (isService) {
			contextPartials.add("service");
		} else {
			contextPartials.add("instance");
		}
	}

	if (isRoot || paths?.file || paths?.folder) {
		contextPartials.add("canRevealFileInOS");
	}

	return Array.from(contextPartials.values()).join(";");
};

/**
	Comparator for sorting an array of explorer items.
*/
export const compareExplorerItemOrder = (a: ExplorerItem, b: ExplorerItem) => {
	const orderA =
		a.treeProvider.metadataProvider.getExplorerOrder(
			a.domInstance.className
		) ?? null;
	const orderB =
		b.treeProvider.metadataProvider.getExplorerOrder(
			b.domInstance.className
		) ?? null;

	if (orderA !== null && orderB !== null) {
		if (orderA !== orderB) {
			return orderA - orderB;
		}
	}

	const labelA = a.label?.toString();
	const labelB = b.label?.toString();
	return labelA && labelB ? labelA.localeCompare(labelB) : 0;
};
