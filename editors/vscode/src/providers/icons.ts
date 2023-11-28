import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";

import { Providers } from ".";

export type IconPack = "None" | "Classic" | "Vanilla2";

type IconPackIcon = { light: vscode.Uri; dark: vscode.Uri };
type IconPackData = Map<string, IconPackIcon>;

type IconPackMetadata = {
	classCount: number;
	classIcons: Record<string, string>;
};

type IconPackMetadatas = {
	light: IconPackMetadata;
	dark: IconPackMetadata;
};

const getAllIconPacks = (): Array<IconPack> => {
	return ["None", "Classic", "Vanilla2"];
};

const readIconPackMetadatas = (extensionPath: string, pack: IconPack): IconPackMetadatas => {
	if (pack === "None") {
		return {
			light: {
				classCount: 0,
				classIcons: {},
			},
			dark: {
				classCount: 0,
				classIcons: {},
			},
		};
	}

	const metaPathLight = path.join(extensionPath, "out", "icons", pack, "light", "metadata.json");
	const metaPathDark = path.join(extensionPath, "out", "icons", pack, "dark", "metadata.json");

	const metaContentsLight = fs.readFileSync(metaPathLight, "utf-8");
	const metaContentsDark = fs.readFileSync(metaPathDark, "utf-8");

	return {
		light: JSON.parse(metaContentsLight),
		dark: JSON.parse(metaContentsDark),
	};
};

const createIconPackData = (
	extensionPath: string,
	pack: IconPack,
	metas: IconPackMetadatas,
): IconPackData => {
	const icons = new Map<string, IconPackIcon>();

	const allClassNames = new Set<string>();
	for (const className of Object.keys(metas.light.classIcons)) {
		allClassNames.add(className);
	}
	for (const className of Object.keys(metas.dark.classIcons)) {
		allClassNames.add(className);
	}

	for (const className of allClassNames) {
		const iconPathLight = path.join(
			extensionPath,
			"out",
			"icons",
			pack,
			"light",
			metas.light.classIcons[className],
		);
		const iconPathDark = path.join(
			extensionPath,
			"out",
			"icons",
			pack,
			"dark",
			metas.dark.classIcons[className],
		);
		icons.set(className, {
			light: vscode.Uri.file(iconPathLight),
			dark: vscode.Uri.file(iconPathDark),
		});
	}

	return icons;
};

export class IconsProvider implements vscode.Disposable {
	private readonly metas: Map<IconPack, IconPackMetadatas> = new Map();
	private readonly icons: Map<IconPack, IconPackData> = new Map();

	constructor(public readonly providers: Providers) {
		for (const pack of getAllIconPacks()) {
			const metas = readIconPackMetadatas(providers.extensionContext.extensionPath, pack);
			const icons = createIconPackData(providers.extensionContext.extensionPath, pack, metas);
			this.metas.set(pack, metas);
			this.icons.set(pack, icons);
		}
	}

	public getClassIcon(pack: IconPack, className: string): IconPackIcon | undefined {
		return this.icons.get(pack)?.get(className) ?? undefined;
	}

	dispose() {}
}
