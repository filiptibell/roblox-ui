import * as vscode from "vscode";
import * as fs from "fs/promises";
import * as path from "path";

import * as fsSync from "fs";

import { extractRojoFileExtension, isInitFilePath } from "./rojo";

type InsertableClassName = "Folder" | "ModuleScript" | "LocalScript" | "Script";

const getInstanceFileName = (
	className: InsertableClassName,
	instanceName: string
): string => {
	if (className === "Script") {
		return `${instanceName}.server.luau`;
	} else if (className === "LocalScript") {
		return `${instanceName}.client.luau`;
	} else {
		return `${instanceName}.luau`;
	}
};

const pathExists = async (path: string) => {
	return new Promise((resolve, _) => {
		fsSync.access(path, fsSync.constants.F_OK, (err) => {
			if (err) {
				resolve(false);
			} else {
				resolve(true);
			}
		});
	});
};

const canCreateInstanceFile = async (
	folderPath: string | null,
	filePath: string | null,
	className: InsertableClassName,
	instanceName: string
): Promise<string | undefined> => {
	if (!folderPath) {
		if (!filePath) {
			return "Missing file path (internal error)";
		}
		if (isInitFilePath(filePath)) {
			folderPath = path.dirname(filePath);
		} else {
			const fileExt = extractRojoFileExtension(filePath);
			if (fileExt) {
				const dirName = path.dirname(filePath);
				const subdirName = path.basename(filePath, `.${fileExt}`);
				folderPath = path.join(dirName, subdirName);
			} else {
				return "Invalid file extension (internal error)";
			}
		}
	}

	if (className === "Folder") {
		const newFolderPath = path.join(folderPath, instanceName);
		if (!(await pathExists(newFolderPath))) {
			return;
		}
		if ((await fs.stat(newFolderPath)).isDirectory()) {
			return `Folder already exists at '${newFolderPath}'`;
		}
	} else {
		const newFileName = getInstanceFileName(className, instanceName);
		const newFilePath = path.join(folderPath, newFileName);
		if (!(await pathExists(newFilePath))) {
			return;
		}
		if ((await fs.stat(newFilePath)).isFile()) {
			return `File already exists at '${newFileName}'`;
		}
	}

	return;
};

export const createNewInstance = async (
	folderPath: string | null,
	filePath: string | null,
	className: InsertableClassName,
	instanceName: string
): Promise<vscode.Uri | null> => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to insert new ${className} instance!` +
				"\nThe selected instance had no folder or file path."
		);
		return null;
	}

	await forceCloseMatchingTextDocuments(folderPath, filePath);

	// If we got a file path that is not an init file, then we
	// need to convert the file into a subfolder + init file
	if (!folderPath) {
		if (!filePath) {
			throw new Error("Unreachable");
		}
		if (isInitFilePath(filePath)) {
			folderPath = path.dirname(filePath);
		} else {
			const fileExt = extractRojoFileExtension(filePath);
			if (fileExt) {
				const dirName = path.dirname(filePath);
				const subdirName = path.basename(filePath, `.${fileExt}`);
				folderPath = path.join(dirName, subdirName);
				await fs.mkdir(folderPath);
				await fs.rename(filePath, `${folderPath}/init.${fileExt}`);
			} else {
				vscode.window.showWarningMessage(
					`Failed to insert new ${className} instance!` +
						"\nThe selected instance had an invalid file extension."
				);
				return null;
			}
		}
	}

	// Make sure the folder path is actually a directory
	const stats = await fs.stat(folderPath);
	if (!stats.isDirectory()) {
		vscode.window.showWarningMessage(
			`Failed to insert new ${className} instance!` +
				"\nThe selected instance had no folder on the filesystem."
		);
		return null;
	}

	// Create the folder or instance file
	if (className === "Folder") {
		const newFolderPath = path.join(folderPath, instanceName);
		await fs.mkdir(newFolderPath);
		return vscode.Uri.file(newFolderPath);
	} else {
		const newFileName = getInstanceFileName(className, instanceName);
		const newFilePath = path.join(folderPath, newFileName);
		await fs.writeFile(newFilePath, "");
		return vscode.Uri.file(newFilePath);
	}
};

export const deleteExistingInstance = async (
	folderPath: string | null,
	filePath: string | null
) => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to delete instance!` +
				"\nThe selected instance had no folder or file path."
		);
		return;
	}

	await forceCloseMatchingTextDocuments(folderPath, filePath);

	// Init files should have both their folder & file paths deleted
	if (folderPath && filePath && isInitFilePath(filePath)) {
		await fs.rm(folderPath, {
			recursive: true,
		});
		return;
	}

	// Folders should also be deleted recursively
	if (folderPath && !filePath) {
		await fs.rm(folderPath, {
			recursive: true,
		});
		return;
	}

	// Files should just be deleted as they are
	// TODO: Delete meta files along with the normal instance file
	// TODO: If the folder the instance is in was using an init file and no longer has
	// any children, try to remove the usage of init files and make it a plain file instead
	if (filePath) {
		await fs.rm(filePath);
		return;
	}

	vscode.window.showWarningMessage(
		`Failed to delete instance!` +
			"\nThe selected instance had an unknown path kind."
	);
};

export const renameExistingInstance = async (
	folderPath: string | null,
	filePath: string | null,
	instanceName: string
) => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to rename instance!` +
				"\nThe selected instance had no folder or file path."
		);
		return;
	}

	// Init files should have their parent folder renamed
	if (folderPath && filePath && isInitFilePath(filePath)) {
		const newFolderPath = path.join(folderPath, "..", instanceName);
		await fs.rename(folderPath, newFolderPath);
		return;
	}

	// Folders should also be renamed the same way
	if (folderPath && !filePath) {
		const newFolderPath = path.join(folderPath, "..", instanceName);
		await fs.rename(folderPath, newFolderPath);
		return;
	}

	// Files should just be renamed as they are
	// TODO: Rename any corresponding meta files, too
	if (filePath) {
		const fileExt = extractRojoFileExtension(filePath);
		if (fileExt) {
			const fileDir = path.dirname(filePath);
			const fileName = `${instanceName}.${fileExt}`;
			await fs.rename(filePath, path.join(fileDir, fileName));
		} else {
			vscode.window.showWarningMessage(
				`Failed to rename instance!` +
					"\nThe selected instance had an unknown path file extension."
			);
		}
		return;
	}

	vscode.window.showWarningMessage(
		`Failed to rename instance!` +
			"\nThe selected instance had an unknown path kind."
	);
};

export const promptNewInstanceCreation = (
	folderPath: string | null,
	filePath: string | null
) => {
	const items = [
		new InstanceInsertItem("Folder"),
		new InstanceInsertItem("ModuleScript"),
		new InstanceInsertItem("LocalScript"),
		new InstanceInsertItem("Script"),
	];
	vscode.window.showQuickPick(items).then((chosen) => {
		if (!chosen) {
			return;
		}
		vscode.window
			.showInputBox({
				prompt: "Type in a name for the new instance",
				value: chosen.className,
				validateInput: async (value: string) => {
					try {
						return await canCreateInstanceFile(
							folderPath,
							filePath,
							chosen.className,
							value
						);
					} catch (e) {
						return `Internal error: ${e}`;
					}
				},
			})
			.then(async (instanceName) => {
				if (!instanceName) {
					return;
				}
				try {
					const createdPath = await createNewInstance(
						folderPath,
						filePath,
						chosen.className,
						instanceName
					);
					if (createdPath) {
						forceShowTextDocument(createdPath);
					}
				} catch (e) {
					vscode.window.showWarningMessage(
						`Failed to insert new instance!` +
							`\nError message:\n${e}`
					);
				}
			});
	});
};

export const promptRenameExistingInstance = (
	folderPath: string | null,
	filePath: string | null
) => {
	const instanceName = "";
	vscode.window
		.showInputBox({
			prompt: "Type in a new name for the instance",
			value: instanceName,
		})
		.then(async (instanceName) => {
			if (!instanceName) {
				return;
			}
			try {
				await renameExistingInstance(
					folderPath,
					filePath,
					instanceName
				);
			} catch (e) {
				vscode.window.showWarningMessage(
					`Failed to rename instance!` + `\nError message:\n${e}`
				);
			}
		});
};

class InstanceInsertItem implements vscode.QuickPickItem {
	label: string;
	description?: string;

	constructor(public readonly className: InsertableClassName) {
		this.label = className;
		// TODO: Descriptions?
		if (className === "Folder") {
		} else if (className === "ModuleScript") {
		} else if (className === "LocalScript") {
		} else if (className === "Script") {
		}
	}
}

const forceCloseMatchingTextDocuments = async (
	folderPath: string | null,
	filePath: string | null
): Promise<[boolean, boolean]> => {
	let wasClosed = false;
	let hadFocus = false;
	for (const doc of vscode.workspace.textDocuments) {
		if (
			(filePath && doc.uri.fsPath === filePath) ||
			(folderPath && doc.uri.fsPath.startsWith(folderPath))
		) {
			const [res1, res2] = await forceCloseTextDocument(doc.uri);
			wasClosed = wasClosed || res1;
			hadFocus = hadFocus || res2;
		}
	}
	return [wasClosed, hadFocus];
};

const forceCloseTextDocument = async (
	uri: vscode.Uri
): Promise<[boolean, boolean]> => {
	// HACK: To properly close an editor that is opened we
	// have to first force show it, force it to be active,
	// and then close the currently active editor
	let wasClosed = false;
	let hadFocus = false;
	for (const doc of vscode.workspace.textDocuments) {
		if (doc.uri.fsPath === uri.fsPath) {
			const textEditor = await vscode.window.showTextDocument(
				doc,
				undefined,
				false
			);
			wasClosed = true;
			hadFocus =
				hadFocus ||
				textEditor.document.uri.fsPath ===
					vscode.window.activeTextEditor?.document.uri.fsPath;
			try {
				textEditor.hide();
			} catch {
				vscode.commands.executeCommand(
					"workbench.action.closeActiveEditor",
					textEditor.document.uri
				);
			}
		}
	}
	return [wasClosed, hadFocus];
};

const forceShowTextDocument = async (uri: vscode.Uri) => {
	if ((await fs.stat(uri.fsPath)).isFile()) {
		const textDoc = await vscode.workspace.openTextDocument(uri);
		await vscode.window.showTextDocument(textDoc);
	}
};
