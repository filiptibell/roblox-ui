import * as path from "path";

const ROJO_PROJECT_EXTENSION = ".project.json";
const ROJO_FILE_EXTENSIONS = [
	"meta.json",
	"model.json",
	"server.luau",
	"server.lua",
	"client.luau",
	"client.lua",
	"luau",
	"lua",
	"rbxm",
	"rbxmx",
	"rbxl",
	"rbxlx",
];

export const extractRojoFileExtension = (filePath: string): string | null => {
	const fileName = path.basename(filePath);
	for (const ext of ROJO_FILE_EXTENSIONS) {
		if (fileName.endsWith(`.${ext}`)) {
			return ext;
		}
	}
	return null;
};

export const isProjectFilePath = (filePath: string): boolean => {
	return filePath.endsWith(ROJO_PROJECT_EXTENSION);
};

export const isBinaryFilePath = (filePath: string): boolean => {
	const fileExt = extractRojoFileExtension(filePath);
	return fileExt === "rbxm" || fileExt === "rbxmx" || fileExt === "rbxl" || fileExt === "rbxlx";
};

export const isInitFilePath = (filePath: string): boolean => {
	if (isBinaryFilePath(filePath)) {
		return false;
	}
	const fileExt = extractRojoFileExtension(filePath);
	if (fileExt) {
		const fileName = path.basename(filePath, `.${fileExt}`);
		return fileName === "init";
	}
	return false;
};
