/* eslint-disable @typescript-eslint/naming-convention */

import * as vscode from "vscode";
import * as path from "path";

import AdmZip = require("adm-zip");

export const readZipFileAsBuffer = async (
	zipFile: Buffer,
	filePath: string
): Promise<Buffer | null> => {
	const zipper = new AdmZip(zipFile);
	return new Promise((resolve) => {
		zipper.readFileAsync(filePath, (data, err) => {
			if (data && !err) {
				resolve(data);
			} else {
				resolve(null);
			}
		});
	});
};

export const readZipFileAsBufferMany = async (
	zipFile: Buffer,
	filePaths: string[]
): Promise<(Buffer | null)[]> => {
	const zipper = new AdmZip(zipFile, { readEntries: true });
	const read = async (filePath: string) => {
		return await (new Promise((resolve) => {
			zipper.readFileAsync(filePath, (data, err) => {
				if (!err) {
					resolve(data);
				} else {
					resolve(null);
				}
			});
		}) as Promise<Buffer | null>);
	};
	const promises = [...filePaths.map(read)];
	return await Promise.all(promises);
};

export const readZipFile = async (
	zipFile: Buffer,
	filePath: string
): Promise<string | null> => {
	const zipper = new AdmZip(zipFile);
	return new Promise((resolve) => {
		zipper.readAsTextAsync(filePath, (data, err) => {
			if (!err) {
				resolve(data);
			} else {
				resolve(null);
			}
		});
	});
};

export const readZipFileMany = async (
	zipFile: Buffer,
	filePaths: string[]
): Promise<(string | null)[]> => {
	const zipper = new AdmZip(zipFile, { readEntries: true });
	const read = async (filePath: string) => {
		return await (new Promise((resolve) => {
			zipper.readAsTextAsync(filePath, (data, err) => {
				if (!err) {
					resolve(data);
				} else {
					resolve(null);
				}
			});
		}) as Promise<string | null>);
	};
	const promises = [...filePaths.map(read)];
	return await Promise.all(promises);
};

export const readZipDirNames = async (
	zipFile: Buffer,
	dirPath: string | void
): Promise<string[]> => {
	const prefix = dirPath
		? dirPath.endsWith("/")
			? dirPath
			: `${dirPath}/`
		: null;
	return new Promise((resolve, reject) => {
		try {
			const zipper = new AdmZip(zipFile);
			const entries = zipper.getEntries();
			const names = new Set<string>([]);
			for (const entry of entries) {
				if (!entry.isDirectory) {
					continue;
				}
				const name = prefix
					? entry.entryName.startsWith(prefix)
						? entry.entryName.slice(prefix.length)
						: null
					: entry.entryName;
				const parts = name ? name.split("/") : null;
				if (parts && parts.length >= 1) {
					names.add(parts[0]);
				}
			}
			resolve(new Array(...names.values()));
		} catch (err) {
			reject(err);
		}
	});
};

export const readZipFileNames = async (
	zipFile: Buffer,
	dirPath: string | void
): Promise<string[]> => {
	const prefix = dirPath
		? dirPath.endsWith("/")
			? dirPath
			: `${dirPath}/`
		: null;
	return new Promise((resolve, reject) => {
		try {
			const zipper = new AdmZip(zipFile);
			const entries = zipper.getEntries();
			const names = new Set<string>([]);
			for (const entry of entries) {
				if (entry.isDirectory) {
					continue;
				}
				const name = prefix
					? entry.entryName.startsWith(prefix)
						? entry.entryName.slice(prefix.length)
						: null
					: entry.entryName;
				const parts = name ? name.split("/") : null;
				if (parts && parts.length === 1) {
					const fileName = parts[0];
					const ext = path.extname(fileName);
					if (ext) {
						names.add(fileName);
					}
				}
			}
			resolve(new Array(...names.values()));
		} catch (err) {
			reject(err);
		}
	});
};
