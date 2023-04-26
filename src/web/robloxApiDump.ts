/* eslint-disable @typescript-eslint/naming-convention */

export type RobloxClassMemberTag =
	| "CanYield"
	| "CustomLuaState"
	| "Deprecated"
	| "Hidden"
	| "NotCreatable"
	| "NotBrowsable"
	| "NotReplicated"
	| "PlayerReplicated"
	| "ReadOnly"
	| "Service"
	| "Yields";

export type RobloxClassMemberType =
	| "Property"
	| "Function"
	| "Event"
	| "Callback";

export type RobloxClassMemberSecurity =
	| "None"
	| "PluginSecurity"
	| "LocalUserSecurity"
	| "RobloxScriptSecurity"
	| "RobloxSecurity"
	| "NotAccessibleSecurity";

export type RobloxClassMemberThreadSafety = "ReadOnly" | "Unsafe";

export type RobloxClassMember = {
	Name: string;
	MemberType: RobloxClassMemberType;
	ThreadSafety: RobloxClassMemberThreadSafety;
	Security:
		| RobloxClassMemberSecurity
		| {
				Read: RobloxClassMemberSecurity;
				Write: RobloxClassMemberSecurity;
		  };
	Category?: string;
	Serialization?: {
		CanLoad?: boolean;
		CanSave?: boolean;
	};
	Tags?: Array<RobloxClassMemberTag>;
	ValueType?: {
		Name: string;
		Category: string;
	};
	ReturnType?: {
		Name: string;
		Category: string;
	};
	Parameters?: Array<{
		Name: string;
		Type: {
			Category: string;
			Name: string;
		};
		Default?: any;
	}>;
};

export type RobloxClass = {
	Name: string;
	Superclass: string;
	MemoryCategory: string;
	Members: Array<RobloxClassMember>;
	Tags: Array<RobloxClassMemberTag>;
};

export type RobloxEnum = {
	Name: string;
	NumItems: number;
	Items: Array<{
		Name: string;
		Value: number;
	}>;
};

export type RobloxApiDump = {
	Classes: Map<string, RobloxClass>;
	Enums: Map<string, RobloxEnum>;
};

export const parseApiDumpFromObject = async (
	data: RobloxApiDump
): Promise<RobloxApiDump> => {
	const classes = new Map<string, RobloxClass>();
	const enums = new Map<string, RobloxEnum>();
	for (const dumpClass of Object.values(data.Classes)) {
		if (dumpClass.Name && dumpClass.Superclass) {
			if (!dumpClass.MemoryCategory) {
				dumpClass.MemoryCategory = "Unknown";
			}
			if (!dumpClass.Members) {
				dumpClass.Members = [];
			}
			if (!dumpClass.Tags) {
				dumpClass.Tags = [];
			}
			classes.set(dumpClass.Name, dumpClass);
		}
	}
	for (const dumpEnum of Object.values(data.Enums)) {
		dumpEnum.NumItems = dumpEnum.Items.length;
		enums.set(dumpEnum.Name, dumpEnum);
	}
	return {
		Classes: classes,
		Enums: enums,
	};
};

export const serializeApiDump = (apiDump: RobloxApiDump): object => {
	return {
		Classes: [...apiDump.Classes.entries()],
		Enums: [...apiDump.Enums.entries()],
	};
};

export const deserializeApiDump = (serialized: unknown): RobloxApiDump => {
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
					"Serialized API dump did not contain set arrays"
				);
			}
		} else {
			throw new Error("Serialized API dump was missing values");
		}
	} else {
		throw new Error("Serialized API dump was not an object");
	}
};
