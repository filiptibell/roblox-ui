import * as vscode from "vscode";
import * as path from "path";

import { SourcemapNode, findPrimaryFilePath } from "../../utils/sourcemap";
import { getClassIconPath } from "../../utils/icons";

import { RojoTreeRoot } from "./root";
import { RojoTreeItem } from "./item";
import { IconsProvider } from "../icons";

export type TreeItemPropChanges = {
	[K in keyof vscode.TreeItem]: vscode.TreeItem[K] | null;
};

export const getNullProps = (): TreeItemPropChanges => {
	return {
		label: null,
		iconPath: null,
		description: null,
		tooltip: null,
		command: null,
		contextValue: null,
		accessibilityInformation: null,
	};
};

export const getLoadingProps = (
	workspacePath: string,
	loadingPath: string | undefined
): TreeItemPropChanges => {
	const wpath = path.parse(workspacePath);
	return {
		...getNullProps(),
		label: "Workspace loading...",
		description: wpath ? wpath.name : null,
		iconPath: new vscode.ThemeIcon("loading~spin"),
		tooltip: loadingPath ? new vscode.MarkdownString(loadingPath) : null,
	};
};

export const getErroredProps = (
	workspacePath: string,
	errorMessage: string
): TreeItemPropChanges => {
	const wpath = path.parse(workspacePath);
	return {
		...getNullProps(),
		label: "Workspace error",
		description: wpath ? wpath.name : null,
		iconPath: new vscode.ThemeIcon("error"),
		tooltip: new vscode.MarkdownString(errorMessage),
	};
};

export const getNodeItemProps = async (
	root: RojoTreeRoot,
	node: SourcemapNode,
	current: RojoTreeItem,
	parent: RojoTreeItem | undefined | null | void
): Promise<TreeItemPropChanges> => {
	const newProps: TreeItemPropChanges = getNullProps();

	// Find folder path and file path to use for props
	const folderPath = node.folderPath
		? path.join(root.workspacePath, node.folderPath)
		: null;
	const filePathRel = findPrimaryFilePath(node);
	const filePath = filePathRel
		? path.join(root.workspacePath, filePathRel)
		: null;
	const fileIsScript = filePath && !filePath.endsWith(".project.json");

	// Set name and icon
	newProps.label = node.name;
	newProps.iconPath = await root.iconsProvider.getPackIcon(
		root.settingsProvider.get("explorer.iconPack"),
		node.className
	);

	// Set description based on settings
	const descriptionPartials: string[] = [];
	const showClassNames = root.settingsProvider.get("explorer.showClassNames");
	if (showClassNames) {
		descriptionPartials.push(node.className);
	}
	const showFilePaths = root.settingsProvider.get("explorer.showFilePaths");
	if (showFilePaths) {
		const fsPathFull = filePath
			? filePath
			: folderPath
			? folderPath
			: undefined;
		const fsPath = fsPathFull
			? fsPathFull.slice(root.workspacePath.length + 1)
			: undefined;
		if (fsPath) {
			descriptionPartials.push(fsPath);
		}
	}
	if (descriptionPartials.length > 0) {
		newProps.description = descriptionPartials.join(" - ");
	}

	// Set context value for menu actions such as copy,
	// paste, insert object, rename, ... to appear correctly
	const contextPartials = new Set();

	if (filePath) {
		if (!parent) {
			contextPartials.add("projectFile");
		} else if (fileIsScript) {
			newProps.command = {
				title: "Open file",
				command: "vscode.open",
				arguments: [vscode.Uri.file(filePath)],
			};
			contextPartials.add("instance");
		}
	} else if (folderPath && !!parent) {
		contextPartials.add("instance");
	}

	if (!!parent?.getFolderPath()) {
		contextPartials.add("canPasteSibling");
	}
	if (folderPath !== null) {
		contextPartials.add("canPasteInto");
		if (contextPartials.has("instance")) {
			contextPartials.add("canInsertObject");
		} else if (contextPartials.has("projectFile")) {
			// DataModel nodes that have a folderPath are safe
			// to add services into, the folder path is a confirmed
			// shared prefix folder where all current services exist
			if (node.className === "DataModel" && node.folderPath) {
				contextPartials.add("canInsertService");
			}
		}
	}

	const info = root.apiDump.Classes.get(node.className);
	const isService = info?.Tags.find((tag) => tag === "Service") !== undefined;
	if (isService) {
		contextPartials.delete("instance");
	} else if (parent && (filePath !== null || folderPath !== null)) {
		contextPartials.add("canMove");
	}

	newProps.contextValue = Array.from(contextPartials.values()).join(";");

	// TODO: Wally integration, read wally file for package(s)

	// Set new resource uri for git/lsp decorations and keeping tree state intact
	newProps.resourceUri = filePath
		? vscode.Uri.file(filePath)
		: folderPath
		? vscode.Uri.file(folderPath)
		: undefined;

	return newProps;
};
