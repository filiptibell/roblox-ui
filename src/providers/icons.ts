import * as vscode from "vscode";
import * as path from "path";

const fs = vscode.workspace.fs;

import { IconPack, downloadIconPack } from "../web/icons";

type IconPackIcon = { light: vscode.Uri; dark: vscode.Uri };
type IconPackData = Map<string, IconPackIcon>;

const stripFileExt = (filePath: string): string => {
	const ext = path.extname(filePath);
	if (ext) {
		return filePath.slice(0, filePath.length - ext.length);
	} else {
		return filePath;
	}
};

const readDir = async (filePath: string): Promise<string[] | null> => {
	const fileUri = vscode.Uri.file(filePath);
	try {
		const entries = await fs.readDirectory(fileUri);
		return [...entries.map((entry) => entry[0])];
	} catch {
		return null;
	}
};

export class IconsProvider {
	private packs: Map<IconPack, IconPackData> = new Map();

	constructor(private readonly context: vscode.ExtensionContext) {}

	private async tryGetCachedIcons(
		pack: IconPack
	): Promise<IconPackData | null> {
		const storagePath = path.join(
			this.context.globalStorageUri.fsPath,
			"iconPacks",
			pack
		);
		const storageDirs = await readDir(storagePath);
		if (storageDirs && storageDirs.length >= 2) {
			if (
				storageDirs.find((entry) => entry === "light") &&
				storageDirs.find((entry) => entry === "dark")
			) {
				const packData = new Map();

				// Read all entries in the file icon dirs to
				// know which class names we have icons made for
				const [filesLight, filesDark] = await Promise.all([
					readDir(path.join(storagePath, "light")),
					readDir(path.join(storagePath, "dark")),
				]);

				// Gather a set of all known filenames
				const fileNames = new Set<string>([]);
				for (const fileName of filesLight!.values()) {
					fileNames.add(fileName);
				}
				for (const fileName of filesDark!.values()) {
					fileNames.add(fileName);
				}

				// Save file icon datas with class name as key
				for (const fileName of fileNames.values()) {
					packData.set(stripFileExt(fileName), {
						light: vscode.Uri.file(
							path.join(storagePath, "light", fileName)
						),
						dark: vscode.Uri.file(
							path.join(storagePath, "dark", fileName)
						),
					});
				}

				// Save and return cached pack data
				this.packs.set(pack, packData);
				return packData;
			}
		}
		return null;
	}

	private async getOrDownloadIconPack(pack: IconPack): Promise<IconPackData> {
		// Return any in-memory cached pack data
		let packData = this.packs.get(pack);
		if (packData) {
			return packData;
		}

		// Check the extension cache directory for already downloaded files
		const cachedIconData = await this.tryGetCachedIcons(pack);
		if (cachedIconData) {
			return cachedIconData;
		}

		// Not cached, we need to download the pack
		const downloadedIcons = await downloadIconPack(pack);

		// Create the directories where we will store our icons
		const storagePath = path.join(
			this.context.globalStorageUri.fsPath,
			"iconPacks",
			pack
		);
		const dirLight = path.join(storagePath, "light");
		const dirDark = path.join(storagePath, "dark");
		await Promise.all([
			fs.createDirectory(vscode.Uri.file(dirLight)),
			fs.createDirectory(vscode.Uri.file(dirDark)),
		]);

		// Save all of the icons to files for caching and usage
		packData = new Map();
		const filePromises = new Array<Thenable<void>>();
		for (const [fileName, fileIcons] of downloadedIcons.entries()) {
			const uriLight = vscode.Uri.file(path.join(dirLight, fileName));
			const uriDark = vscode.Uri.file(path.join(dirDark, fileName));
			filePromises.push(fs.writeFile(uriLight, fileIcons.light));
			filePromises.push(fs.writeFile(uriDark, fileIcons.dark));
			packData.set(stripFileExt(fileName), {
				light: uriLight,
				dark: uriDark,
			});
		}
		await Promise.all(filePromises);

		// Save and return new pack data
		this.packs.set(pack, packData);
		return packData;
	}

	/**
	 * Get file paths for the given icon pack, and download the icon pack if necessary.
	 */
	public async getPack(pack: IconPack): Promise<IconPackData> {
		return await this.getOrDownloadIconPack(pack);
	}

	/**
	 * Get file paths for the given icon, or download it if necessary.
	 */
	public async getPackIcon(
		pack: IconPack,
		className: string
	): Promise<IconPackIcon> {
		const packData = await this.getOrDownloadIconPack(pack);
		return packData.get(className) ?? packData.get(className)!;
	}

	dispose() {}
}
