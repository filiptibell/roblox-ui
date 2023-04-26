import * as vscode from "vscode";
import * as path from "path";

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
		collapsibleState: null,
		contextValue: null,
		accessibilityInformation: null,
	};
};

export const getLoadingProps = (workspacePath: string): TreeItemPropChanges => {
	const wpath = path.parse(workspacePath);
	return {
		...getNullProps(),
		label: "Workspace loading...",
		description: wpath ? wpath.name : null,
		iconPath: new vscode.ThemeIcon("loading~spin"),
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

// TODO: Move functionality for props from the RojoTreeItem constructor into this
