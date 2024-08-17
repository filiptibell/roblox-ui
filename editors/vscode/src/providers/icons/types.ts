import * as vscode from "vscode";

export type IconPack = "None" | "Classic" | "Vanilla2" | "RobloxCustom";

export type IconPackIcon = { light: vscode.Uri; dark: vscode.Uri };
export type IconPackData = Map<string, IconPackIcon>;

export type IconPackMetadata = {
	classCount: number;
	classIcons: Record<string, string>;
};

export type IconPackMetadatas = {
	light: IconPackMetadata;
	dark: IconPackMetadata;
};
