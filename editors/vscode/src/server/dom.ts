type DomNotificationAdded = {
	parentId?: string;
	childId: string;
};

type DomNotificationRemoved = {
	parentId?: string;
	childId: string;
};

type DomNotificationChanged = {
	id: string;
	className?: string;
	name?: string;
};

export type DomNotification =
	| null
	| { kind: "Added"; data: DomNotificationAdded }
	| { kind: "Removed"; data: DomNotificationRemoved }
	| { kind: "Changed"; data: DomNotificationChanged };

export type DomInstance = {
	id: string;
	className: string;
	name: string;
	children?: string[];
};

export type DomRootRequest = void;
export type DomRootResponse = null | DomInstance;

export type DomGetRequest = { id: string };
export type DomGetResponse = null | DomInstance;

export type DomChildrenRequest = { id: string };
export type DomChildrenResponse = DomInstance[];
