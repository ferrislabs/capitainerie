import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { tanstackApi } from '#/api'

export function monthWindow(now: Date): { from: string; to: string } {
	const from = new Date(now.getFullYear(), now.getMonth(), 1)
	return { from: from.toISOString(), to: now.toISOString() }
}

export function useMonthCost() {
	const window = useMemo(() => monthWindow(new Date()), [])
	return useQuery(
		tanstackApi.get('/api/v1/analytics/costs', {
			query: { by: 'day', ...window },
		}).queryOptions,
	)
}
