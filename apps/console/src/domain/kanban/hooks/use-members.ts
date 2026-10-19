import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export function useMembers(options: { enabled?: boolean } = {}) {
	return useQuery({
		...tanstackApi.get('/api/v1/members').queryOptions,
		enabled: options.enabled ?? true,
	})
}

export function memberNames(
	members: ReadonlyArray<{ subject: string; name: string }>,
): ReadonlyMap<string, string> {
	return new Map(members.map((member) => [member.subject, member.name]))
}
