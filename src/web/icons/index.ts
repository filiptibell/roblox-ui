import * as vscode from "vscode";

import memoize = require("memoizee");

export type IconPack = "Classic" | "Vanilla 2.1";

import { download as downloadClassic } from "./classic";
import { download as downloadVanilla } from "./vanilla";

const downloadPack = async (
	pack: IconPack,
	progressCallback: (progress: number) => any
) => {
	if (pack === "Classic") {
		return await downloadClassic(progressCallback);
	} else if (pack === "Vanilla 2.1") {
		return await downloadVanilla(progressCallback);
	} else {
		throw new Error(`Invalid icon pack name: ${pack}`);
	}
};

const downloadPackWithProgressBar = async (pack: IconPack) => {
	return await vscode.window.withProgress(
		{
			location: vscode.ProgressLocation.Notification,
			title: `Downloading icon pack '${pack}'`,
		},
		async (report) => {
			return await downloadPack(pack, (progress) => {
				report.report({ increment: progress * 100 });
			});
		}
	);
};

export const downloadIconPack: typeof downloadPackWithProgressBar = memoize(
	downloadPackWithProgressBar,
	{ promise: true }
);
