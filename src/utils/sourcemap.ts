import * as vscode from "vscode";
import * as path from "path";

export type SourcemapNode = {
	name: string;
	className: string;
	filePaths?: string[];
	children?: SourcemapNode[];
	parent?: SourcemapNode;
};

const sourcemapSetParents = (
	node: SourcemapNode,
	parent: SourcemapNode | undefined
) => {
	if (parent) {
		node.parent = parent;
	}
	if (node.children) {
		for (const child of node.children.values()) {
			sourcemapSetParents(child, node);
		}
	}
};

export const parseSourcemap = (txt: string): SourcemapNode => {
	const sourcemap = JSON.parse(txt);
	sourcemapSetParents(sourcemap, undefined);
	return sourcemap;
};

export const getSourcemapNodeTreeOrder = (node: SourcemapNode): number => {
	if (
		node.className === "Script" ||
		node.className === "LocalScript" ||
		node.className === "ModuleScript"
	) {
		return 3;
	} else if (node.className === "Folder") {
		return 2;
	} else {
		return 1;
	}
};

export const findFilePath = (
	workspaceRoot: string,
	node: SourcemapNode
): [string | null, boolean | null] => {
	if (node.filePaths) {
		let filePath = node.filePaths.find((filePath) => {
			return (
				filePath.endsWith(".project.json") ||
				filePath.endsWith(".lua") ||
				filePath.endsWith(".luau")
			);
		});
		if (filePath) {
			return [
				path.join(workspaceRoot, filePath),
				!filePath.endsWith(".project.json"),
			];
		}
	}
	return [null, null];
};
