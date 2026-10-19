import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { tanstackApi } from '#/api'

export function monthStart(now: Date): string {
	return new Date(
		Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), 1),
	).toISOString()
}

export function useMonthCost() {
	const from = useMemo(() => monthStart(new Date()), [])
	return useQuery(
		tanstackApi.get('/api/v1/analytics/costs', {
			query: { by: 'day', from },
		}).queryOptions,
	)
}
