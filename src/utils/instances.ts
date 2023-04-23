import * as vscode from "vscode";
import * as fs from "fs/promises";
import * as path from "path";

import { extractRojoFileExtension, isInitFilePath } from "./rojo";

type InsertableClassName = "Folder" | "ModuleScript" | "LocalScript" | "Script";

export const createNewInstance = async (
	folderPath: string | null,
	filePath: string | null,
	className: InsertableClassName,
	instanceName: string
) => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to insert new ${className} instance!` +
				"\nThe selected instance had no folder or file path."
		);
		return;
	}

	// If we got a file path that is not an init file, then we
	// need to convert the file into a subfolder + init file
	if (!folderPath) {
		if (!filePath) {
			vscode.window.showWarningMessage(
				`Failed to insert new ${className} instance!` +
					"\nThe selected instance had no file path."
			);
			return;
		}
		const dirName = path.dirname(filePath);
		const fileExt = extractRojoFileExtension(filePath);
		vscode.window.showInformationMessage(filePath);
		if (fileExt) {
			const subdirName = path.basename(filePath, `.${fileExt}`);
			folderPath = path.join(dirName, subdirName);
			await fs.mkdir(folderPath);
			await fs.rename(filePath, `${folderPath}/init.${fileExt}`);
		} else {
			vscode.window.showWarningMessage(
				`Failed to insert new ${className} instance!` +
					"\nThe selected instance had an invalid file extension."
			);
			return;
		}
	}

	// Make sure the folder path is actually a directory
	const stats = await fs.stat(folderPath);
	if (!stats.isDirectory()) {
		vscode.window.showWarningMessage(
			`Failed to insert new ${className} instance!` +
				"\nThe selected instance had no folder on the filesystem."
		);
		return;
	}

	// Create the folder or instance file
	if (className === "Folder") {
		await fs.mkdir(path.join(folderPath, instanceName));
	} else {
		let fileName;
		if (className === "Script") {
			fileName = `${instanceName}.server.luau`;
		} else if (className === "LocalScript") {
			fileName = `${instanceName}.client.luau`;
		} else {
			fileName = `${instanceName}.luau`;
		}
		await fs.writeFile(path.join(folderPath, fileName), "");
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
			})
			.then(async (instanceName) => {
				if (!instanceName) {
					return;
				}
				try {
					await createNewInstance(
						folderPath,
						filePath,
						chosen.className,
						instanceName
					);
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
