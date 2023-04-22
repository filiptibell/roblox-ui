import * as path from "path";

export const getClassIconPath = (className: string): string => {
	return path.join(__dirname, "..", "..", "icons", `${className}.png`);
};
