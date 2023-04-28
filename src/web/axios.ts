import * as axios from "axios";

import memoize = require("memoizee");

const download = async (
	url: string,
	progressCallback: (progress: number) => any,
	responseType: axios.ResponseType | void
): Promise<any> => {
	progressCallback(0);
	const result = await (new Promise((resolve, reject) => {
		axios.default
			.get(url, {
				responseType: responseType ?? "text",
				onDownloadProgress(progressEvent) {
					if (progressEvent.progress) {
						progressCallback(progressEvent.progress);
					} else {
						progressCallback(0);
					}
				},
			})
			.then((res) => resolve(res.data))
			.catch((err) => reject(err));
	}) as Promise<any>);
	progressCallback(1);
	if (responseType === "arraybuffer") {
		return Buffer.from(result, "binary");
	} else {
		return result;
	}
};

export const downloadWithProgress: typeof download = memoize(download, {
	promise: true,
});
