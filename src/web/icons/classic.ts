import * as path from "path";

import { readZipFileAsBufferMany, readZipFileNames } from "../../utils/zip";
import { downloadWithProgress } from "../axios";

const PACK_BASE_URL = "https://devforum.roblox.com/uploads/short-url";
const PACK_ZIP_URL = `${PACK_BASE_URL}/vHjNEH4jUjBlz9tl8Yetp6T1b99.zip`;

export const download = async () => {
	const zip = await downloadWithProgress(
		PACK_ZIP_URL,
		() => {},
		"arraybuffer"
	);

	const fileNames = await readZipFileNames(zip, "RobloxCustom/standard");
	const filePaths = fileNames.map((name) => `RobloxCustom/standard/${name}`);
	const fileContents = await readZipFileAsBufferMany(zip, filePaths);

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
