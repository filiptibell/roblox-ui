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
	Classes: Array<RobloxClass>;
	Enums: Array<RobloxEnum>;
};

export const parseApiDumpFromObject = async (
	data: RobloxApiDump
): Promise<RobloxApiDump> => {
	const classes = new Array<RobloxClass>();
	const enums = new Array<RobloxEnum>();
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
			classes.push(dumpClass);
		}
	}
	for (const dumpEnum of Object.values(data.Enums)) {
		dumpEnum.NumItems = dumpEnum.Items.length;
		enums.push(dumpEnum);
	}
	return {
		Classes: classes,
		Enums: enums,
	};
};
