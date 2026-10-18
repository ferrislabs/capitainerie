import type { Schemas } from '#/api/api.client'

export type ProviderKind = Schemas.ProviderKindDto

export const PROVIDER_KINDS: ReadonlyArray<ProviderKind> = [
	'anthropic',
	'mistral',
	'openai_compatible',
]

export interface ProviderInfo {
	label: string
	shortLabel: string
	help: string
	keyRequired: boolean
	usesBaseUrl: boolean
}

export const PROVIDER_INFO: Record<ProviderKind, ProviderInfo> = {
	anthropic: {
		label: 'Claude (Anthropic)',
		shortLabel: 'Claude',
		help: 'Créez une clé d’API sur console.anthropic.com, rubrique API Keys.',
		keyRequired: true,
		usesBaseUrl: false,
	},
	mistral: {
		label: 'Mistral',
		shortLabel: 'Mistral',
		help: 'Créez une clé d’API sur console.mistral.ai, rubrique API Keys.',
		keyRequired: true,
		usesBaseUrl: false,
	},
	openai_compatible: {
		label: 'Compatible OpenAI',
		shortLabel: 'Compatible OpenAI',
		help: 'Un endpoint qui parle l’API OpenAI : Ollama, vLLM… Un Ollama local ne demande pas de clé.',
		keyRequired: false,
		usesBaseUrl: true,
	},
}

export const BASE_URL_PLACEHOLDER = 'http://localhost:4120/v1'

export const ADMIN_ONLY_PROVIDERS =
	'Seuls les administrateurs de l’organisation peuvent modifier les fournisseurs.'
export const ADMIN_ONLY_MODELS =
	'Seuls les administrateurs de l’organisation peuvent modifier les modèles par rôle.'

export function providerLabel(kind: ProviderKind): string {
	return PROVIDER_INFO[kind].label
}

export function validateApiKey(
	kind: ProviderKind,
	apiKey: string,
): string | undefined {
	if (PROVIDER_INFO[kind].keyRequired && apiKey.trim() === '') {
		return 'La clé d’API est obligatoire.'
	}
	return undefined
}

export function validateBaseUrl(
	kind: ProviderKind,
	baseUrl: string,
): string | undefined {
	if (!PROVIDER_INFO[kind].usesBaseUrl) return undefined
	const value = baseUrl.trim()
	if (value === '') return 'L’URL de base est obligatoire.'
	try {
		const url = new URL(value)
		if (url.protocol !== 'http:' && url.protocol !== 'https:') {
			return 'L’URL doit commencer par http:// ou https://.'
		}
	} catch {
		return 'URL invalide.'
	}
	return undefined
}
