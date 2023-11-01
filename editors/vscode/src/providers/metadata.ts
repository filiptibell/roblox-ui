import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";

type Classes = {
	classCount: number;
	classDatas: Record<string, ClassData>;
};

type ClassData = {
	name: string;
	description?: string;
	documentationUrl?: string;
	isService?: boolean;
	isDeprecated?: boolean;
	notBrowsable?: boolean;
	notCreatable?: boolean;
};

type Reflection = {
	classes: Record<string, ReflectionClass>;
	enums: Record<string, ReflectionEnum>;
};

type ReflectionClass = {
	name: string;
	summary?: string;
	values?: Record<string, ReflectionValue>;
};

type ReflectionEnum = {
	name: string;
	summary?: string;
	values?: Record<string, ReflectionValue>;
	items: Array<ReflectionEnumItem>;
};

type ReflectionEnumItem = {
	name: string;
	summary?: string;
	values?: Record<string, ReflectionValue>;
};

type ReflectionValue = string | number | boolean;

export class MetadataProvider implements vscode.Disposable {
	private classes: Classes;
	private reflection: Reflection;

	constructor(private readonly context: vscode.ExtensionContext) {
		const classesPath = path.join(
			context.extensionPath,
			"out",
			"data",
			"classes.json"
		);
		const reflectionPath = path.join(
			context.extensionPath,
			"out",
			"data",
			"reflection.json"
		);

		const classesContents = fs.readFileSync(classesPath, "utf-8");
		const reflectionContents = fs.readFileSync(reflectionPath, "utf-8");

		this.classes = JSON.parse(classesContents);
		this.reflection = JSON.parse(reflectionContents);
	}

	public getClassData(className: string): ClassData | null {
		let classData = this.classes.classDatas[className];
		if (typeof classData === "object") {
			return classData;
		} else {
			return null;
		}
	}

	public getExplorerOrder(className: string): number | null {
		let classReflection = this.reflection.classes[className];
		if (typeof classReflection === "object") {
			let classOrder = classReflection.values?.ExplorerOrder;
			if (typeof classOrder === "number") {
				return classOrder;
			}
		}
		return null;
	}

	public getInsertableClassNames(): Array<string> {
		let classNames = new Array();

		for (const className of Object.keys(this.classes.classDatas)) {
			classNames.push(className);
		}

		return classNames;
	}

	dispose() {}
}
