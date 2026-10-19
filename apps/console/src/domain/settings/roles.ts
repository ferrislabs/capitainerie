import type { Schemas } from '#/api/api.client'
import { PROVIDER_KINDS, type ProviderKind } from '#/domain/settings/providers'

export type AgentRole = Exclude<keyof Schemas.ModelAssignmentsDto, 'executor'>
export type ModelRef = Schemas.ModelRefDto
export type ModelAssignments = Schemas.ModelAssignmentsDto
export type AssignmentDraft = Partial<Record<AgentRole, ModelRef>> & {
	executor?: string | null
}

export const TRANSCRIBER_PROVIDERS: ReadonlyArray<ProviderKind> = [
	'mistral',
	'openai_compatible',
]

export interface RoleInfo {
	label: string
	description: string
	capability: keyof Schemas.CapabilitiesResponse
}

export type RequiredRole = Exclude<AgentRole, 'transcriber'>

export const AGENT_ROLES: ReadonlyArray<RequiredRole> = [
	'extractor',
	'judge',
	'writer',
	'embeddings',
]

export const TRANSCRIBER_ROLE = 'transcriber' satisfies AgentRole

export const ROLE_INFO: Record<AgentRole, RoleInfo> = {
	extractor: {
		label: 'Extracteur',
		description: 'Lit le transcript et en extrait les actions.',
		capability: 'chat',
	},
	judge: {
		label: 'Juge de doublons',
		description:
			'Compare chaque action aux cartes existantes pour écarter les doublons.',
		capability: 'chat',
	},
	writer: {
		label: 'Rédacteur',
		description: 'Rédige le titre et la description des cartes proposées.',
		capability: 'chat',
	},
	embeddings: {
		label: 'Embeddings',
		description: 'Vectorise les cartes pour retrouver les doublons proches.',
		capability: 'embeddings',
	},
	transcriber: {
		label: 'Transcripteur',
		description:
			'Transforme un audio déposé en texte, que vous relisez avant de lancer le run. Facultatif : sans lui, seuls les fichiers texte sont acceptés.',
		capability: 'transcription',
	},
}

export const EXECUTOR_INFO = {
	label: 'Exécutant',
	description:
		'Claude Code, sur les cartes confiées. Modèles Claude uniquement.',
} as const

export const EXECUTOR_MODEL_PREFIX = 'claude-'

export function executorModels(
	models: ReadonlyArray<Schemas.ModelResponse>,
): Array<Schemas.ModelResponse> {
	return models.filter(
		(model) =>
			model.provider === 'anthropic' &&
			model.capabilities.chat &&
			model.id.startsWith(EXECUTOR_MODEL_PREFIX),
	)
}

export function sameExecutor(
	a: string | null | undefined,
	b: string | null | undefined,
): boolean {
	return (a ?? null) === (b ?? null)
}

export function compatibleModels(
	models: ReadonlyArray<Schemas.ModelResponse>,
	role: AgentRole,
): Array<Schemas.ModelResponse> {
	const capability = ROLE_INFO[role].capability
	return models.filter(
		(model) =>
			model.capabilities[capability] &&
			(role !== TRANSCRIBER_ROLE ||
				TRANSCRIBER_PROVIDERS.includes(model.provider)),
	)
}

export interface ProviderGroup<T> {
	provider: ProviderKind
	items: Array<T>
}

export function groupByProvider<T extends { provider: ProviderKind }>(
	items: ReadonlyArray<T>,
): Array<ProviderGroup<T>> {
	return PROVIDER_KINDS.map((provider) => ({
		provider,
		items: items.filter((item) => item.provider === provider),
	})).filter((group) => group.items.length > 0)
}

export function modelRefKey(ref: ModelRef): string {
	return `${ref.provider}:${ref.model}`
}

export function parseModelRefKey(key: string): ModelRef | undefined {
	const separator = key.indexOf(':')
	if (separator <= 0) return undefined
	const provider = key.slice(0, separator)
	const model = key.slice(separator + 1)
	if (!model || !(PROVIDER_KINDS as ReadonlyArray<string>).includes(provider)) {
		return undefined
	}
	return { provider: provider as ProviderKind, model }
}

export function sameModelRef(
	a: ModelRef | undefined,
	b: ModelRef | undefined,
): boolean {
	return a?.provider === b?.provider && a?.model === b?.model
}

export function completeAssignments(
	draft: AssignmentDraft,
): ModelAssignments | undefined {
	const { extractor, judge, writer, embeddings, transcriber, executor } = draft
	if (!extractor || !judge || !writer || !embeddings) return undefined
	return {
		extractor,
		judge,
		writer,
		embeddings,
		transcriber: transcriber ?? null,
		...(executor ? { executor } : {}),
	}
}

export function draftFrom(
	saved: Schemas.ModelAssignmentsDto | null,
): AssignmentDraft {
	if (saved === null) return {}
	const { transcriber, ...rest } = saved
	return { ...rest, ...(transcriber ? { transcriber } : {}) }
}

export function catalogRef(model: Schemas.ModelResponse): ModelRef {
	return { provider: model.provider, model: model.id }
}
