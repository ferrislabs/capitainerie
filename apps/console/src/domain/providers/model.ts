import type { Schemas } from '#/api/api.client'

export type ProviderKind = Schemas.ProviderKindDto

export interface ProviderSpec {
	kind: ProviderKind
	name: string
	modelPlaceholder: string
	needsBaseUrl: boolean
	keyRequired: boolean
	baseUrlHelp?: string
}

export const PROVIDER_SPECS: ReadonlyArray<ProviderSpec> = [
	{
		kind: 'anthropic',
		name: 'Anthropic',
		modelPlaceholder: 'claude-sonnet-5-5',
		needsBaseUrl: false,
		keyRequired: true,
	},
	{
		kind: 'mistral',
		name: 'Mistral',
		modelPlaceholder: 'mistral-large-latest',
		needsBaseUrl: false,
		keyRequired: true,
	},
	{
		kind: 'openai',
		name: 'OpenAI',
		modelPlaceholder: 'gpt-5.5',
		needsBaseUrl: false,
		keyRequired: true,
	},
	{
		kind: 'google',
		name: 'Google Gemini',
		modelPlaceholder: 'gemini-3.8-flash',
		needsBaseUrl: false,
		keyRequired: true,
	},
	{
		kind: 'openai_compatible',
		name: 'Compatible OpenAI',
		modelPlaceholder: 'gemini-3-flash-preview',
		needsBaseUrl: true,
		keyRequired: false,
		baseUrlHelp:
			'ex. Gemini : https://generativelanguage.googleapis.com/v1beta/openai',
	},
]

export function specOf(kind: ProviderKind): ProviderSpec {
	return PROVIDER_SPECS.find((spec) => spec.kind === kind) as ProviderSpec
}

export function maskedPreview(preview: string): string {
	return preview ? `••••${preview}` : 'Aucune clé'
}

export interface ProviderDraft {
	apiKey: string
	baseUrl: string
	model: string
	isDefault: boolean
}

export function validateDraft(
	spec: ProviderSpec,
	draft: ProviderDraft,
): string | null {
	if (spec.keyRequired && draft.apiKey.trim() === '') {
		return 'La clé API est obligatoire.'
	}
	if (spec.needsBaseUrl && !/^https?:\/\/[^\s/]+/i.test(draft.baseUrl.trim())) {
		return "L'URL de base doit commencer par https://"
	}
	if (draft.model.trim() === '') {
		return 'Le modèle par défaut est obligatoire.'
	}
	return null
}

export function buildPutBody(
	spec: ProviderSpec,
	draft: ProviderDraft,
): Schemas.PutUserProviderRequest {
	return {
		api_key: draft.apiKey.trim(),
		default_model: draft.model.trim(),
		is_default: draft.isDefault,
		...(spec.needsBaseUrl ? { base_url: draft.baseUrl.trim() } : {}),
	}
}

function statusOf(error: unknown): number | undefined {
	return (error as { status?: number } | null)?.status
}

const FORBIDDEN = "Vous n'avez pas le droit de gérer vos fournisseurs."

export function describeSaveError(error: unknown): string {
	switch (statusOf(error)) {
		case 422:
			return "Paramètres refusés : vérifiez la clé API, l'URL de base (obligatoire pour un fournisseur compatible OpenAI) et le modèle."
		case 403:
			return FORBIDDEN
		case 404:
			return "Ce fournisseur n'existe plus."
		default:
			return "L'enregistrement a échoué. Réessayez."
	}
}

export function describeDeleteError(error: unknown): string {
	switch (statusOf(error)) {
		case 404:
			return "Ce fournisseur n'est pas connecté."
		case 403:
			return FORBIDDEN
		default:
			return 'La suppression a échoué. Réessayez.'
	}
}

export function describeTestError(error: unknown): string {
	switch (statusOf(error)) {
		case 404:
			return "Ce fournisseur n'est pas connecté."
		case 403:
			return FORBIDDEN
		default:
			return 'Le test a échoué. Réessayez.'
	}
}
