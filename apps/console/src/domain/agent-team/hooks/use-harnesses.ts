import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

export function useHarnesses() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/harnesses').queryOptions,
		enabled: Boolean(accessToken),
		staleTime: Number.POSITIVE_INFINITY,
	})
}
