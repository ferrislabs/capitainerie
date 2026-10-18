export interface ExecuteOutput {
	pullRequest?: string
	branch?: string
	noChanges: boolean
	droppedToolCalls: number
	error?: string
}

function text(record: Record<string, unknown>, key: string) {
	const value = record[key]
	return typeof value === 'string' && value !== '' ? value : undefined
}

export function safeLink(value: string | undefined): string | undefined {
	if (!value) return undefined
	try {
		const { protocol } = new URL(value)
		return protocol === 'https:' || protocol === 'http:' ? value : undefined
	} catch {
		return undefined
	}
}

export function parseExecuteOutput(output: unknown): ExecuteOutput | undefined {
	if (typeof output !== 'object' || output === null) return undefined
	const record = output as Record<string, unknown>
	const dropped = record.dropped_tool_calls
	return {
		pullRequest: safeLink(text(record, 'pull_request')),
		branch: text(record, 'branch'),
		noChanges: record.no_changes === true,
		droppedToolCalls:
			typeof dropped === 'number' && Number.isFinite(dropped) && dropped > 0
				? Math.floor(dropped)
				: 0,
		error: text(record, 'error'),
	}
}
