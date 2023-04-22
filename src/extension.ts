import * as fs from "fs";
import * as vscode from "vscode";

import { RojoTreeProvider, SourcemapNode } from "./provider";

const workspaceDestructors: Map<string, Function> = new Map();

const connectWorkspace = (
	folder: vscode.WorkspaceFolder,
	treeDataProvider: RojoTreeProvider
) => {
	const workspacePath = folder.uri.fsPath;
	const sourcemapPath = `${workspacePath}/sourcemap.json`;

	const fileWatcher = vscode.workspace.createFileSystemWatcher(sourcemapPath);
	fileWatcher.onDidChange(() => {
		fs.readFile(
			sourcemapPath,
			{
				encoding: "utf8",
			},
			(err, txt) => {
				if (err) {
					throw err;
				} else {
					let sourcemap: SourcemapNode = JSON.parse(txt);
					treeDataProvider.update(workspacePath, sourcemap);
				}
			}
		);
	});

	fs.readFile(
		sourcemapPath,
		{
			encoding: "utf8",
		},
		(err, txt) => {
			if (err) {
				throw err;
			} else {
				let sourcemap: SourcemapNode = JSON.parse(txt);
				treeDataProvider.update(workspacePath, sourcemap);
			}
		}
	);

	workspaceDestructors.set(workspacePath, () => {
		treeDataProvider.delete(workspacePath);
		fileWatcher.dispose();
	});
};

const disconnectWorkspace = (folder: vscode.WorkspaceFolder) => {
	const workspacePath = folder.uri.fsPath;
	const destroy = workspaceDestructors.get(workspacePath);
	if (destroy !== undefined) {
		destroy();
		workspaceDestructors.delete(workspacePath);
	}
};

export function activate(context: vscode.ExtensionContext) {
	const treeDataProvider = new RojoTreeProvider(context);
	context.subscriptions.push(
		vscode.window.registerTreeDataProvider("rojoExplorer", treeDataProvider)
	);

	context.subscriptions.push(
		vscode.workspace.onDidChangeWorkspaceFolders((event) => {
			for (const addedFolder of event.added) {
				connectWorkspace(addedFolder, treeDataProvider);
			}
			for (const removedFolder of event.removed) {
				disconnectWorkspace(removedFolder);
			}
		})
	);

	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (workspaceFolders) {
		workspaceFolders.forEach((folder) => {
			connectWorkspace(folder, treeDataProvider);
		});
	}
}

export function deactivate() {
	let workspacePaths = [...workspaceDestructors.keys()];
	workspacePaths.forEach((workspacePath) => {
		const destroy = workspaceDestructors.get(workspacePath);
		if (destroy !== undefined) {
			destroy();
			workspaceDestructors.delete(workspacePath);
		}
	});
}
