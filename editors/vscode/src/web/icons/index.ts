import * as vscode from "vscode";

import memoize = require("memoizee");

import { RobloxApiDump, RobloxReflectionMetadata } from "../roblox";

import { download as downloadClassic } from "./classic";
import { download as downloadVanilla } from "./vanilla";

export type IconPack = "Classic" | "Vanilla2";

const downloadPack = async (
	pack: IconPack,
	apiDump: RobloxApiDump,
	reflection: RobloxReflectionMetadata,
	progressCallback: (progress: number) => any
) => {
	if (pack === "Classic") {
		return await downloadClassic(apiDump, reflection, progressCallback);
	} else if (pack === "Vanilla2") {
		return await downloadVanilla(apiDump, reflection, progressCallback);
	} else {
		throw new Error(`Invalid icon pack name: ${pack}`);
	}
};

const downloadPackWithProgressBar = async (
	pack: IconPack,
	apiDump: RobloxApiDump,
	reflection: RobloxReflectionMetadata
) => {
	return await vscode.window.withProgress(
		{
			location: vscode.ProgressLocation.Notification,
			title: `Downloading icon pack '${pack}'`,
		},
		async (report) => {
			return await downloadPack(pack, apiDump, reflection, (progress) => {
				report.report({ increment: progress * 100 });
			});
		}
	);
};

export const downloadIconPack: typeof downloadPackWithProgressBar = memoize(
	downloadPackWithProgressBar,
	{ promise: true }
);
