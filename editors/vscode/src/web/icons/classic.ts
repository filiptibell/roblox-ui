import * as path from "path";

import { readZipFileAsBufferMany, readZipFileNames } from "../../utils/zip";
import { downloadWithProgress } from "../axios";
import { RobloxApiDump, RobloxReflectionMetadata } from "../roblox";

const PACK_BASE_URL = "https://devforum.roblox.com/uploads/short-url";
const PACK_ZIP_URL = `${PACK_BASE_URL}/vHjNEH4jUjBlz9tl8Yetp6T1b99.zip`;

export const download = async (
	apiDump: RobloxApiDump,
	reflection: RobloxReflectionMetadata,
	progressCallback: (progress: number) => any
) => {
	const zip = await downloadWithProgress(
		PACK_ZIP_URL,
		(progress) => {
			progressCallback(progress * 0.9);
		},
		"arraybuffer"
	);

	const fileNames = await readZipFileNames(zip, "RobloxCustom/standard");
	progressCallback(0.95);

	const filePaths = fileNames.map((name) => `RobloxCustom/standard/${name}`);
	const fileContents = await readZipFileAsBufferMany(zip, filePaths);
	progressCallback(1);

	const icons = new Map<string, { light: Buffer; dark: Buffer }>();
	for (const [index, fileName] of fileNames.entries()) {
		const icon = fileContents[index];
		const ext = path.extname(fileName);
		if (icon && ext) {
			icons.set(fileName, {
				light: icon,
				dark: icon,
			});
		}
	}
	return icons;
};
