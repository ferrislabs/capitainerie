import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { tanstackApi } from '#/api'
import { type CostDimension, costsWindow } from '#/domain/agents/dimensions'

export function useCosts(by: CostDimension) {
	const window = useMemo(() => costsWindow(new Date()), [])
	return useQuery(
		tanstackApi.get('/api/v1/analytics/costs', {
			query: { by, ...window },
		}).queryOptions,
	)
}
