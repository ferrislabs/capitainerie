import { useQuery } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { tanstackApi } from '#/api'

export const JOURNAL_POLL_INTERVAL_MS = 5000

export function journalRefetchInterval(polling: boolean): number | false {
	return polling ? JOURNAL_POLL_INTERVAL_MS : false
}

function useFinalRead(polling: boolean, refetch: () => unknown) {
	const wasPolling = useRef(false)
	useEffect(() => {
		if (wasPolling.current && !polling) void refetch()
		wasPolling.current = polling
	}, [polling, refetch])
}

export function useCardIntegrationActions(
	cardId: string | undefined,
	polling: boolean,
) {
	const query = useQuery({
		...tanstackApi.get('/api/v1/cards/{card_id}/integration-actions', {
			path: { card_id: cardId ?? '' },
		}).queryOptions,
		enabled: cardId !== undefined,
		refetchInterval: journalRefetchInterval(polling),
	})
	useFinalRead(polling, query.refetch)
	return query
}

export function useRunIntegrationActions(
	runId: string | undefined,
	polling: boolean,
) {
	const query = useQuery({
		...tanstackApi.get('/api/v1/runs/{run_id}/integration-actions', {
			path: { run_id: runId ?? '' },
		}).queryOptions,
		enabled: runId !== undefined,
		refetchInterval: journalRefetchInterval(polling),
	})
	useFinalRead(polling, query.refetch)
	return query
}
