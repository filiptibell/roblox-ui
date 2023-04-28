/* eslint-disable @typescript-eslint/naming-convention */

const xml2js = require("xml2js");

import { readZipFileAsBuffer } from "../../utils/zip";

type Dictionary<T> = {
	[key: string]: T;
};

export type RobloxReflectionClassFunction = {
	Name: string;
	summary?: string;
	ScriptContext?: string;
	Deprecated?: boolean;
};

export type RobloxReflectionClassEvent = {
	Name: string;
	summary?: string;
	ScriptContext?: string;
	Deprecated?: boolean;
};

export type RobloxReflectionClassProperty = {
	Name: string;
	summary?: string;
	UIMinimum?: number;
	UIMaximum?: number;
	UINumTicks?: number;
};

export type RobloxReflectionClass = {
	Name: string;
	summary?: string;

	Browsable?: boolean;
	Deprecated?: boolean;
	Insertable?: boolean;

	ExplorerOrder?: number;
	ExplorerImageIndex?: number;

	PreferredParent?: string;
	ClassCategory?: string;

	Properties?: Array<RobloxReflectionClassProperty>;
	YieldFunctions?: Array<RobloxReflectionClassFunction>;
	Functions?: Array<RobloxReflectionClassFunction>;
	Callbacks?: Array<RobloxReflectionClassFunction>;
	Events?: Array<RobloxReflectionClassEvent>;
};

export type RobloxReflectionEnumItem = {
	Name: string;
	summary?: string;
	Browsable?: boolean;
	Deprecated?: boolean;
};

export type RobloxReflectionEnum = {
	Name: string;
	summary?: string;
	Browsable?: boolean;
	Deprecated?: boolean;
	Items: Array<RobloxReflectionEnumItem>;
};

export type RobloxReflectionMetadata = {
	Classes: Map<string, RobloxReflectionClass>;
	Enums: Map<string, RobloxReflectionEnum>;
};

const FILENAME_METADATA = "ReflectionMetadata.xml";

const PROP_COERCED_TYPES: Dictionary<string | null> = {
	Browsable: "bool",
	Deprecated: "bool",
	Insertable: "bool",
	ExplorerOrder: "number",
	ExplorerImageIndex: "number",
	UIMinimum: "number",
	UIMaximum: "number",
	UINumTicks: "number",
};

export const parseReflectionMetadataFromRobloxStudioZip = async (
	buf: Buffer
): Promise<RobloxReflectionMetadata> => {
	// Get file buffer from zipped data
	const data = await readZipFileAsBuffer(buf, FILENAME_METADATA);
	// Parse xml string into js object
	return new Promise((resolve, reject) => {
		xml2js.parseString(data, (err: any, obj: any) => {
			if (err) {
				reject(err);
			} else {
				const root = obj.roblox.Item;
				const metadataClasses: Map<string, RobloxReflectionClass> =
					new Map();
				const metadataEnums: Map<string, RobloxReflectionEnum> =
					new Map();
				// Parse classes and set infos
				parseMetadataClasses(root[0].Item).forEach((metadataClass) => {
					metadataClasses.set(metadataClass.Name, metadataClass);
				});
				// Parse enums and set infos
				parseMetadataEnums(root[1].Item).forEach((metadataEnum) => {
					metadataEnums.set(metadataEnum.Name, metadataEnum);
				});
				// Create full reflection
				resolve({
					Classes: metadataClasses,
					Enums: metadataEnums,
				});
			}
		});
	});
};

export const serializeReflection = (
	reflection: RobloxReflectionMetadata
): object => {
	return {
		Classes: [...reflection.Classes.entries()],
		Enums: [...reflection.Enums.entries()],
	};
};

export const deserializeReflection = (
	serialized: unknown
): RobloxReflectionMetadata => {
	if (typeof serialized === "object" && serialized !== null) {
		if ("Classes" in serialized && "Enums" in serialized) {
			if (
				Array.isArray(serialized.Classes) &&
				Array.isArray(serialized.Enums)
			) {
				return {
					Classes: new Map(serialized["Classes"]),
					Enums: new Map(serialized["Enums"]),
				};
			} else {
				throw new Error(
					"Serialized reflection metadata did not contain set arrays"
				);
			}
		} else {
			throw new Error(
				"Serialized reflection metadata was missing values"
			);
		}
	} else {
		throw new Error("Serialized reflection metadata was not an object");
	}
};

const transformPropertyValue = (
	valueType: string,
	valueName: string,
	valueString: string
) => {
	const forcedType = PROP_COERCED_TYPES[valueName];
	if (forcedType) {
		valueType = forcedType;
	}
	if (valueType === "string") {
		return valueString;
	} else if (valueType === "bool") {
		return valueString.toString() === "true";
	} else if (valueType === "number") {
		const valueNum = parseFloat(valueString);
		if (typeof valueNum === "number") {
			if (!isNaN(valueNum)) {
				return valueNum;
			}
		}
	}
	return null;
};

const transformXmlToJs = (
	data: any
): Dictionary<string | boolean | number | null> => {
	const result: Dictionary<string | boolean | number | null> = {};
	if (data !== undefined) {
		for (const propData of data.values()) {
			for (const [valueType, valueArray] of Object.entries(propData)) {
				const valueArr: any = valueArray;
				for (const valueData of valueArr) {
					if (!valueData["$"]) {
						continue;
					}
					const propName: string = valueData["$"].name;
					const propValue: string = valueData["_"];
					result[propName] = transformPropertyValue(
						valueType,
						propName,
						propValue
					);
				}
			}
		}
	}
	return result;
};

const getReflectionItemNameFromClass = (itemType: string): string => {
	return itemType.replace("ReflectionMetadata", "");
};

const getReflectionItems = (items: any) => {
	const result = [];
	if (items !== undefined) {
		for (const item of items.values()) {
			result.push(transformXmlToJs(item.Properties));
		}
	}
	return result;
};

const parseMetadataClasses = (
	classesArg: any
): Array<RobloxReflectionClass> => {
	const classes: any = [];
	for (const xmlInfo of classesArg.values()) {
		const full: Dictionary<any> = transformXmlToJs(xmlInfo.Properties);
		if (xmlInfo.Item !== undefined) {
			for (const item of xmlInfo.Item.values()) {
				const key = getReflectionItemNameFromClass(item["$"].class);
				full[key] = getReflectionItems(item.Item);
			}
		}
		classes.push(full);
	}
	return classes;
};

const parseMetadataEnums = (enumsArg: any): Array<RobloxReflectionEnum> => {
	const enums: any = [];
	for (const xmlInfo of enumsArg.values()) {
		const full: Dictionary<any> = transformXmlToJs(xmlInfo.Properties);
		const enumItems = [];
		if (xmlInfo.Item !== undefined) {
			for (const enumItemXml of xmlInfo.Item.values()) {
				enumItems.push(transformXmlToJs(enumItemXml.Properties));
			}
		}
		full.Items = enumItems;
		enums.push(full);
	}
	return enums;
};
