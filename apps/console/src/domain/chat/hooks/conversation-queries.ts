import type { QueryKey } from '@tanstack/react-query'

const CONVERSATIONS_PATH = '/api/v1/conversations'

export function isConversationQuery(key: QueryKey): boolean {
	const head: unknown = key[0]
	if (typeof head !== 'object' || head === null || !('_id' in head)) {
		return false
	}
	const id = (head as { _id: unknown })._id
	return typeof id === 'string' && id.startsWith(CONVERSATIONS_PATH)
}
