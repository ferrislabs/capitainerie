import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { mutationErrorMessage } from '#/domain/settings/errors'
import { invalidateProviderData } from '#/domain/settings/hooks/use-settings-queries'
import {
	ADMIN_ONLY_PROVIDERS,
	PROVIDER_INFO,
	type ProviderKind,
} from '#/domain/settings/providers'

export interface ProviderSecretInput {
	apiKey: string
	baseUrl: string
}

interface SaveVariables {
	kind: ProviderKind
	baseUrl: string | null
	/** Rend la clé une seule fois puis l'oublie : l'état de la mutation ne la garde pas. */
	takeKey: () => string
}

export function oneShot(value: string): () => string {
	let held: string | null = value
	return () => {
		const taken = held ?? ''
		held = null
		return taken
	}
}

interface MutateOptions {
	onSuccess?: () => void
}

export function useSaveProvider() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		mutationKey: [{ method: 'put', path: '/api/v1/providers/{kind}' }],
		mutationFn: ({ kind, baseUrl, takeKey }: SaveVariables) => {
			const body: Schemas.PutProviderRequest = PROVIDER_INFO[kind].usesBaseUrl
				? { api_key: takeKey(), base_url: baseUrl }
				: { api_key: takeKey() }
			return api.put('/api/v1/providers/{kind}', { path: { kind }, body })
		},
		onSuccess: () => invalidateProviderData(queryClient),
	})

	return {
		save: (
			kind: ProviderKind,
			input: ProviderSecretInput,
			options?: MutateOptions,
		) => {
			const baseUrl = input.baseUrl.trim()
			mutation.mutate(
				{ kind, baseUrl: baseUrl || null, takeKey: oneShot(input.apiKey) },
				{ onSuccess: () => options?.onSuccess?.() },
			)
		},
		reset: mutation.reset,
		isPending: mutation.isPending,
		variables: mutation.variables,
		savedKind: mutation.isSuccess ? mutation.variables?.kind : undefined,
		errorMessage: mutationErrorMessage(
			mutation.error,
			ADMIN_ONLY_PROVIDERS,
			'Enregistrement impossible.',
		),
	}
}

export function useDeleteProvider() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		mutationKey: [{ method: 'delete', path: '/api/v1/providers/{kind}' }],
		mutationFn: async (kind: ProviderKind) => {
			await api.delete('/api/v1/providers/{kind}', { path: { kind } })
		},
		onSuccess: () => invalidateProviderData(queryClient),
	})

	return {
		remove: (kind: ProviderKind, options?: MutateOptions) =>
			mutation.mutate(kind, { onSuccess: () => options?.onSuccess?.() }),
		reset: mutation.reset,
		isPending: mutation.isPending,
		errorMessage: mutationErrorMessage(
			mutation.error,
			ADMIN_ONLY_PROVIDERS,
			'Suppression impossible.',
		),
	}
}

export type TestOutcome =
	| { ok: true; modelCount: number }
	| { ok: false; error: string }

export function testOutcome(data: Schemas.TestProviderResponse): TestOutcome {
	return data.ok
		? { ok: true, modelCount: data.models.length }
		: {
				ok: false,
				error: data.error || 'Le fournisseur a refusé la connexion.',
			}
}

export function useTestProvider() {
	const queryClient = useQueryClient()
	const [results, setResults] = useState<
		Partial<Record<ProviderKind, TestOutcome>>
	>({})

	function record(kind: ProviderKind, outcome: TestOutcome) {
		setResults((previous) => ({ ...previous, [kind]: outcome }))
	}

	const mutation = useMutation({
		mutationKey: [{ method: 'post', path: '/api/v1/providers/{kind}/test' }],
		mutationFn: (kind: ProviderKind) =>
			api.post('/api/v1/providers/{kind}/test', { path: { kind } }),
		onSuccess: (data, kind) => {
			record(kind, testOutcome(data))
			return invalidateProviderData(queryClient)
		},
		onError: (error, kind) => {
			record(kind, {
				ok: false,
				error:
					mutationErrorMessage(
						error,
						ADMIN_ONLY_PROVIDERS,
						'Test impossible.',
					) ?? 'Test impossible.',
			})
		},
	})

	return {
		test: (kind: ProviderKind) => mutation.mutate(kind),
		forget: (kind: ProviderKind) =>
			setResults((previous) => {
				const next = { ...previous }
				delete next[kind]
				return next
			}),
		pendingKind: mutation.isPending ? mutation.variables : undefined,
		results,
	}
}
