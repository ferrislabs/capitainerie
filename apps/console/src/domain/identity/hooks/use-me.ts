import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

export function useMe() {
	const accessToken = useAccessToken()

	return useQuery({
		...tanstackApi.get('/api/v1/me').queryOptions,
		enabled: Boolean(accessToken),
	})
}
