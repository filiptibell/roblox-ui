import * as vscode from "vscode";

const fs = vscode.workspace.fs;

export const pathMetadata = async (path: string) => {
	try {
		const stats = await fs.stat(vscode.Uri.file(path));
		return {
			exists: stats.type !== vscode.FileType.Unknown,
			isFile: stats.type === vscode.FileType.File,
			isDir: stats.type === vscode.FileType.Directory,
		};
	} catch {
		return {
			exists: false,
			isFile: false,
			isDir: false,
		};
	}
};
