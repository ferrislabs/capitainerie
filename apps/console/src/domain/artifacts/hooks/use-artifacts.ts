import { useInfiniteQuery, useQuery } from '@tanstack/react-query'
import { api, tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

const ARTIFACTS_PATH = '/api/v1/artifacts'
const PAGE_SIZE = 48
const CARD_LIMIT = 50

export type ArtifactFilters = Partial<{
	kind: Schemas.ArtifactKindDto
	agent_id: string
}>

export function useArtifacts(filters: ArtifactFilters) {
	const accessToken = useAccessToken()
	return useInfiniteQuery({
		queryKey: [{ _id: ARTIFACTS_PATH, infinite: true, query: filters }],
		queryFn: ({ pageParam, signal }) =>
			api.get(ARTIFACTS_PATH, {
				query: {
					...filters,
					limit: PAGE_SIZE,
					...(pageParam ? { cursor: pageParam } : {}),
				},
				overrides: { signal },
			}),
		initialPageParam: '',
		getNextPageParam: (last: Schemas.ArtifactListResponse) =>
			last.next_cursor ?? undefined,
		enabled: Boolean(accessToken),
	})
}

export function useCardArtifacts(cardId: string) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get(ARTIFACTS_PATH, {
			query: { card_id: cardId, limit: CARD_LIMIT },
		}).queryOptions,
		enabled: Boolean(accessToken) && cardId !== '',
	})
}

export function useArtifact(artifactId: string | null) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/artifacts/{artifact_id}', {
			path: { artifact_id: artifactId ?? '' },
		}).queryOptions,
		enabled: Boolean(accessToken) && artifactId !== null,
	})
}
