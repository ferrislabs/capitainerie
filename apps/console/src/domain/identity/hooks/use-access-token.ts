import { useSyncExternalStore } from 'react'
import { authStore } from '#/store/auth.store'

export function useAccessToken(): string | null {
	return useSyncExternalStore(
		(onChange) => authStore.subscribe(onChange).unsubscribe,
		() => authStore.state.accessToken,
		() => null,
	)
}
