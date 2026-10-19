import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

interface ListOptions {
	query?: string
	archived?: boolean
	enabled?: boolean
}

function listQuery({ query, archived }: ListOptions) {
	const text = query?.trim()
	return {
		...(text ? { q: text } : {}),
		...(archived ? { archived: true } : {}),
	}
}

export function useClients(options: ListOptions = {}) {
	return useQuery({
		...tanstackApi.get('/api/v1/clients', { query: listQuery(options) })
			.queryOptions,
		enabled: options.enabled ?? true,
	})
}

export function useProjects(options: ListOptions & { clientId?: string } = {}) {
	return useQuery({
		...tanstackApi.get('/api/v1/projects', {
			query: {
				...listQuery(options),
				...(options.clientId ? { client_id: options.clientId } : {}),
			},
		}).queryOptions,
		enabled: options.enabled ?? true,
	})
}

export function useClient(clientId: string) {
	return useQuery(
		tanstackApi.get('/api/v1/clients/{client_id}', {
			path: { client_id: clientId },
		}).queryOptions,
	)
}

export function useProject(projectId: string) {
	return useQuery(
		tanstackApi.get('/api/v1/projects/{project_id}', {
			path: { project_id: projectId },
		}).queryOptions,
	)
}
