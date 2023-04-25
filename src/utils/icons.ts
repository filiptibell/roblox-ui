import * as path from "path";
import * as fs from "fs";

import { RobloxApiDump } from "../web/robloxApiDump";

const DEFAULT_CLASS_NAME: string = "Instance";
const ICON_CLASS_FILE_CACHE: Map<string, boolean> = new Map();

const getFullClassIconFilePath = (className: string): string => {
	return path.join(__dirname, "..", "..", "icons", `${className}.png`);
};

const getClassIconExists = (className: string): boolean => {
	const cached = ICON_CLASS_FILE_CACHE.get(className);
	if (cached !== undefined) {
		return cached;
	} else {
		const filePath = getFullClassIconFilePath(className);
		const fileExists = fs.existsSync(filePath);
		ICON_CLASS_FILE_CACHE.set(className, fileExists);
		return fileExists;
	}
};

export const getClassIconPath = (
	apiDump: RobloxApiDump,
	className: string
): string => {
	let current: string | null = className;
	while (current && current.length > 0) {
		if (getClassIconExists(current)) {
			return getFullClassIconFilePath(current);
		} else {
			const apiClass = apiDump.Classes.get(current);
			if (apiClass && apiClass.Superclass) {
				current = apiClass.Superclass;
			} else {
				break;
			}
		}
	}
	return getFullClassIconFilePath(DEFAULT_CLASS_NAME);
};
