import * as vscode from "vscode";
import * as path from "path";

import { SourcemapNode, findPrimaryFilePath } from "../../utils/sourcemap";

import { RojoTreeRoot } from "./root";
import { RojoTreeItem } from "./item";

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
	loadingPath: string | undefined,
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
	errorMessage: string,
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
	parent: RojoTreeItem | undefined | null,
): Promise<TreeItemPropChanges> => {
	const newProps: TreeItemPropChanges = getNullProps();
	const classData = root.metadataProvider.getClassData(node.className);

	// Find folder path and file path to use for props
	const folderPath = node.folderPath ? path.join(root.workspacePath, node.folderPath) : null;
	const filePathRel = findPrimaryFilePath(node);
	const filePath = filePathRel ? path.join(root.workspacePath, filePathRel) : null;
	const fileIsScript = filePath && !filePath.endsWith(".project.json");

	// Set name, icon, description (tooltip)
	newProps.label = node.name;
	newProps.iconPath = root.iconsProvider.getClassIcon(
		root.settingsProvider.get("explorer.iconPack"),
		node.className,
	);
	let tooltip = `### ${node.name}`;
	if (node.className !== node.name) {
		tooltip += "\n\n";
		tooltip += `#### ${node.className}`;
	}
	const desc = classData?.description ?? null;
	if (typeof desc === "string" && desc.length > 0) {
		tooltip += "\n\n";
		tooltip += desc;
	}
	const link = classData?.documentationUrl ?? null;
	if (typeof link === "string" && link.length > 0) {
		tooltip += "\n\n";
		tooltip += `[Learn More $(link-external)](${link})`;
	}
	tooltip += "\n";
	const tooltipMarkdown = new vscode.MarkdownString(tooltip);
	tooltipMarkdown.supportThemeIcons = true;
	tooltipMarkdown.supportHtml = true;
	tooltipMarkdown.isTrusted = true;
	newProps.tooltip = tooltipMarkdown;

	// Set description based on settings
	const descriptionPartials: string[] = [];
	const showPackageVersion = root.settingsProvider.get("wally.showPackageVersion");
	if (showPackageVersion && node.wallyVersion) {
		descriptionPartials.push(node.wallyVersion);
	}
	const showClassNames = root.settingsProvider.get("explorer.showClassNames");
	if (showClassNames) {
		descriptionPartials.push(node.className);
	}
	const showFilePaths = root.settingsProvider.get("explorer.showFilePaths");
	if (showFilePaths) {
		const fsPathFull = filePath ? filePath : folderPath ? folderPath : undefined;
		const fsPath = fsPathFull ? fsPathFull.slice(root.workspacePath.length + 1) : undefined;
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
			if (folderPath && node.className === "DataModel") {
				contextPartials.add("canInsertService");
			}
		} else if (fileIsScript) {
			newProps.command = {
				title: "Open file",
				command: "vscode.open",
				arguments: [vscode.Uri.file(filePath)],
			};
			contextPartials.add("instance");
			contextPartials.add("canInsertObject");
		}
	} else if (folderPath && !!parent) {
		contextPartials.add("instance");
		contextPartials.add("canInsertObject");
	}

	const isService = classData?.isService ?? false;
	if (isService) {
		contextPartials.delete("instance");
	} else if (parent && (filePath || folderPath)) {
		contextPartials.add("canMove");
	}

	if (parent?.getFolderPath()) {
		contextPartials.add("canPasteSibling");
	}
	if (!isService && folderPath && contextPartials.has("instance")) {
		contextPartials.add("canPasteInto");
	}

	newProps.contextValue = Array.from(contextPartials.values()).join(";");

	// Set new resource uri for git/lsp decorations and keeping tree state intact
	newProps.resourceUri = filePath
		? vscode.Uri.file(filePath)
		: folderPath
		  ? vscode.Uri.file(folderPath)
		  : undefined;

	return newProps;
};
