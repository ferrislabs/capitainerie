export function taskPathId(id: string): string {
	return id.toLowerCase()
}

export function parseTaskId(pathId: string): string {
	return pathId.toUpperCase()
}
