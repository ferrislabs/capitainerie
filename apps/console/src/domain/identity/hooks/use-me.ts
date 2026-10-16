import { useQuery } from '@tanstack/react-query'
import { useSyncExternalStore } from 'react'
import { tanstackApi } from '#/api'
import { authStore } from '#/store/auth.store'

export function useMe() {
	const accessToken = useSyncExternalStore(
		(onChange) => authStore.subscribe(onChange).unsubscribe,
		() => authStore.state.accessToken,
		() => null,
	)

	return useQuery({
		...tanstackApi.get('/api/v1/me').queryOptions,
		enabled: Boolean(accessToken),
	})
}
