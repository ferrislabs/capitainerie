import {
	keepPreviousData,
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

export const ROUTINES_ID = '/api/v1/routines'

export function invalidateRoutines(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith(ROUTINES_ID) ?? false
		},
	})
}

export function useRoutines(archived = false, enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/routines', { query: { archived } })
			.queryOptions,
		enabled: Boolean(accessToken) && enabled,
	})
}

export function useRoutine(routineId: string, enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/routines/{routine_id}', {
			path: { routine_id: routineId },
		}).queryOptions,
		enabled: Boolean(accessToken) && Boolean(routineId) && enabled,
	})
}

export function useRoutineRuns(
	routineId: string,
	limit: number,
	offset: number,
) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/routines/{routine_id}/runs', {
			path: { routine_id: routineId },
			query: { limit, offset },
		}).queryOptions,
		enabled: Boolean(accessToken),
		placeholderData: keepPreviousData,
	})
}

export function useCreateRoutine() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/routines').mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useUpdateRoutine() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/routines/{routine_id}')
			.mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useArchiveRoutine() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/routines/{routine_id}/archive')
			.mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useLaunchRoutine() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/routines/{routine_id}/launch')
			.mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useAddTrigger() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/routines/{routine_id}/triggers')
			.mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useUpdateTrigger() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation(
			'patch',
			'/api/v1/routines/{routine_id}/triggers/{trigger_id}',
		).mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useRemoveTrigger() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation(
			'delete',
			'/api/v1/routines/{routine_id}/triggers/{trigger_id}',
		).mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}

export function useRotateTriggerToken() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation(
			'post',
			'/api/v1/routines/{routine_id}/triggers/{trigger_id}/rotate-token',
		).mutationOptions,
		onSuccess: () => invalidateRoutines(client),
	})
}
