const SHORT_LENGTH = 8

export function taskPathId(id: string): string {
	return id.toLowerCase()
}

export function shortTaskId(id: string): string {
	return id.slice(0, SHORT_LENGTH)
}
