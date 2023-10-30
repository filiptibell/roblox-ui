import * as cp from "child_process";

export type ChildProcessResult = {
	ok: boolean;
	status: number | null;
	stdout: string;
	stderr: string;
};

/**
 * Spawns a child process and kills it if it
 * has not exited after the given timeout.
 *
 * This promise will always resolve, never reject.
 *
 * The default timeout is 5 seconds (5000 milliseconds).
 */
export const spawnWithTimeout = (
	command: string,
	args: string[],
	options?: cp.SpawnOptionsWithoutStdio | undefined,
	timeoutMillis?: number | undefined
): Promise<ChildProcessResult> => {
	return new Promise((resolve) => {
		let proc = cp.spawn(command, args, options);

		let stdout = "";
		let stderr = "";

		proc.stdout.on("data", (data) => {
			stdout += data.toString("utf8");
		});
		proc.stderr.on("data", (data) => {
			stderr += data.toString("utf8");
		});

		let timeoutHandle = setTimeout(
			() => {
				resolve({
					ok: false,
					status: 99,
					stdout: "",
					stderr: `Command timed out: '${command}'`,
				});
				proc.kill();
			},
			timeoutMillis ? timeoutMillis : 5_000
		);

		proc.on("close", (code) => {
			clearTimeout(timeoutHandle);
			resolve({
				ok: code === null || code === 0,
				status: code,
				stdout,
				stderr,
			});
		});
	});
};
