import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useSyncExternalStore } from 'react'
import type { RealtimeState } from '#/lib/realtime'
import { realtimeStore, realtimeUrl, startRealtime } from '#/lib/realtime'
import { getRuntimeConfig } from '#/lib/runtime-config'
import { authStore } from '#/store/auth.store'

export function useRealtimeState(): RealtimeState {
	return useSyncExternalStore(
		(onChange) => realtimeStore.subscribe(onChange).unsubscribe,
		() => realtimeStore.state.state,
		() => 'offline',
	)
}

export function useRealtime(): void {
	const queryClient = useQueryClient()
	const authenticated = useSyncExternalStore(
		(onChange) => authStore.subscribe(onChange).unsubscribe,
		() => authStore.state.isAuthenticated,
		() => false,
	)

	useEffect(() => {
		if (!authenticated) return
		const base = getRuntimeConfig().apiUrl || window.location.origin
		return startRealtime({ url: realtimeUrl(base), queryClient })
	}, [authenticated, queryClient])
}
