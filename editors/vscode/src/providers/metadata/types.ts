export type Classes = {
	classCount: number;
	classDatas: Record<string, ClassData>;
};

export type ClassData = {
	name: string;
	category?: string;
	description?: string;
	documentationUrl?: string;
	preferredParent?: string;
	isService?: boolean;
	isDeprecated?: boolean;
	notBrowsable?: boolean;
	notCreatable?: boolean;
};

export type Reflection = {
	classes: Record<string, ReflectionClass>;
	enums: Record<string, ReflectionEnum>;
};

export type ReflectionClass = {
	name: string;
	summary?: string;
	values?: Record<string, ReflectionValue>;
};

export type ReflectionEnum = {
	name: string;
	summary?: string;
	values?: Record<string, ReflectionValue>;
	items: Array<ReflectionEnumItem>;
};

export type ReflectionEnumItem = {
	name: string;
	summary?: string;
	values?: Record<string, ReflectionValue>;
};

export type ReflectionValue = string | number | boolean;
