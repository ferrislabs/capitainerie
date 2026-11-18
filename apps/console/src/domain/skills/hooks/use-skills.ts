import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { useMemo } from 'react'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

function invalidateSkills(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/skills') ?? false
		},
	})
}

export function useSkills(archived = false, enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/skills', { query: { archived } }).queryOptions,
		enabled: Boolean(accessToken) && enabled,
	})
}

/** Slugs des compétences actives, pour proposer des suggestions. */
export function useSkillCatalog(enabled = true): ReadonlyArray<string> {
	const items = useSkills(false, enabled).data?.items
	return useMemo(() => (items ?? []).map((s) => s.slug), [items])
}

export function useCreateSkill() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/skills').mutationOptions,
		onSuccess: () => invalidateSkills(client),
	})
}

export function useUpdateSkill() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/skills/{skill_id}')
			.mutationOptions,
		onSuccess: () => invalidateSkills(client),
	})
}

export function useArchiveSkill() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/skills/{skill_id}/archive')
			.mutationOptions,
		onSuccess: () => invalidateSkills(client),
	})
}
