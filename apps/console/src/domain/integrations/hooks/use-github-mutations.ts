import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import {
	connectErrorMessage,
	createAppErrorMessage,
	managementErrorMessage,
} from '#/domain/integrations/errors'
import { GITHUB_QUERY_KEY } from '#/domain/integrations/hooks/use-github-integration'

const REPOSITORIES_QUERY_KEY = [
	{ _id: '/api/v1/integrations/github/repositories' },
] as const

export function useStartGithubInstall() {
	const mutation = useMutation({
		mutationKey: [
			{ method: 'get', path: '/api/v1/integrations/github/install' },
		],
		mutationFn: () => api.get('/api/v1/integrations/github/install'),
		onSuccess: ({ url }) => window.location.assign(url),
	})

	return {
		start: () => mutation.mutate(),
		reset: mutation.reset,
		isPending: mutation.isPending,
		errorMessage: managementErrorMessage(
			mutation.error,
			'Impossible de démarrer la connexion à GitHub.',
		),
	}
}

export function useDisconnectGithub() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		mutationKey: [{ method: 'delete', path: '/api/v1/integrations/github' }],
		mutationFn: () => api.delete('/api/v1/integrations/github'),
		onSuccess: () =>
			Promise.all([
				queryClient.invalidateQueries({ queryKey: GITHUB_QUERY_KEY }),
				queryClient.removeQueries({ queryKey: REPOSITORIES_QUERY_KEY }),
			]),
	})

	return {
		remove: (options?: { onSuccess?: () => void }) =>
			mutation.mutate(undefined, {
				onSuccess: () => options?.onSuccess?.(),
			}),
		reset: mutation.reset,
		isPending: mutation.isPending,
		errorMessage: managementErrorMessage(
			mutation.error,
			'Impossible de déconnecter GitHub.',
		),
	}
}

export interface GithubCallbackParams {
	installationId: number
	code: string
	state: string
}

export function useConnectGithub() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		mutationKey: [
			{ method: 'post', path: '/api/v1/integrations/github/callback' },
		],
		mutationFn: ({ installationId, code, state }: GithubCallbackParams) =>
			api.post('/api/v1/integrations/github/callback', {
				body: { installation_id: installationId, code, state },
			}),
		onSuccess: () =>
			Promise.all([
				queryClient.invalidateQueries({ queryKey: GITHUB_QUERY_KEY }),
				queryClient.removeQueries({ queryKey: REPOSITORIES_QUERY_KEY }),
			]),
	})

	return {
		connect: mutation.mutate,
		isPending: mutation.isPending,
		isSuccess: mutation.isSuccess,
		errorMessage: connectErrorMessage(mutation.error),
	}
}

export function useCreateGithubManifest() {
	const mutation = useMutation({
		mutationKey: [
			{ method: 'post', path: '/api/v1/integrations/github/app/manifest' },
		],
		mutationFn: (owner: Schemas.ManifestOwner) =>
			api.post('/api/v1/integrations/github/app/manifest', {
				body: { owner },
			}),
	})

	return {
		create: (
			owner: Schemas.ManifestOwner,
			options: { onSuccess: (manifest: Schemas.ManifestResponse) => void },
		) => mutation.mutate(owner, { onSuccess: options.onSuccess }),
		reset: mutation.reset,
		isPending: mutation.isPending,
		errorMessage: managementErrorMessage(
			mutation.error,
			'Impossible de préparer la création de l’application GitHub.',
		),
	}
}

export interface GithubAppCallbackParams {
	code: string
	state: string
}

export function useCreateGithubApp() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		mutationKey: [{ method: 'post', path: '/api/v1/integrations/github/app' }],
		mutationFn: ({ code, state }: GithubAppCallbackParams) =>
			api.post('/api/v1/integrations/github/app', { body: { code, state } }),
		onSuccess: () =>
			queryClient.invalidateQueries({ queryKey: GITHUB_QUERY_KEY }),
	})

	return {
		create: mutation.mutate,
		isPending: mutation.isPending,
		isSuccess: mutation.isSuccess,
		errorMessage: createAppErrorMessage(mutation.error),
	}
}

export function useDeleteGithubApp() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		mutationKey: [
			{ method: 'delete', path: '/api/v1/integrations/github/app' },
		],
		mutationFn: () => api.delete('/api/v1/integrations/github/app'),
		onSuccess: () =>
			Promise.all([
				queryClient.invalidateQueries({ queryKey: GITHUB_QUERY_KEY }),
				queryClient.removeQueries({ queryKey: REPOSITORIES_QUERY_KEY }),
			]),
	})

	return {
		remove: (options: {
			onSuccess: (deleted: Schemas.GithubAppResponse) => void
		}) => mutation.mutate(undefined, { onSuccess: options.onSuccess }),
		reset: mutation.reset,
		isPending: mutation.isPending,
		errorMessage: managementErrorMessage(
			mutation.error,
			'Impossible de supprimer l’application GitHub.',
		),
	}
}
