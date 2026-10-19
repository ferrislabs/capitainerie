import type { QueryKey } from '@tanstack/react-query'

const DIRECTORY_PATHS = ['/api/v1/clients', '/api/v1/projects'] as const

export function isDirectoryQuery(key: QueryKey): boolean {
	const head: unknown = key[0]
	if (typeof head !== 'object' || head === null || !('_id' in head)) {
		return false
	}
	const id = (head as { _id: unknown })._id
	return (
		typeof id === 'string' &&
		DIRECTORY_PATHS.some((path) => id.startsWith(path))
	)
}
