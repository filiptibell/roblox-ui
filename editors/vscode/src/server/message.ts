export type RpcMessageKind = "Request" | "Response";

export type RpcMessageData = {
	id: number;
	method: string;
	value?: any;
};

export type RpcMessage = {
	kind: RpcMessageKind;
	data: RpcMessageData;
};

export const validateRpcMessage = (
	messageString: string
): { valid: true; message: RpcMessage } | { valid: false; err: string } => {
	const message = JSON.parse(messageString);
	if (typeof message !== "object") {
		return {
			valid: false,
			err: `message must be an object, got ${typeof message}`,
		};
	}
	if (typeof message.kind !== "string") {
		return {
			valid: false,
			err: `message.kind must be a string, got ${typeof message.kind}`,
		};
	}
	if (typeof message.data !== "object") {
		return {
			valid: false,
			err: `message.data must be a object, got ${typeof message.data}`,
		};
	}
	if (typeof message.data.id !== "number") {
		return {
			valid: false,
			err: `message.data.id must be a number, got ${typeof message.data
				.id}`,
		};
	}
	if (typeof message.data.method !== "string") {
		return {
			valid: false,
			err: `message.data.method must be a string, got ${typeof message
				.data.method}`,
		};
	}
	return {
		valid: true,
		message,
	};
};

export const createRpcRequest = (
	method: string,
	id: number,
	request?: any
): RpcMessage => {
	return {
		kind: "Request",
		data: {
			id,
			method,
			value: request,
		},
	};
};

export const respondToRpcMessage = (
	message: RpcMessage,
	response?: any
): RpcMessage => {
	return {
		kind: "Response",
		data: {
			id: message.data.id,
			method: message.data.method,
			value: response,
		},
	};
};
