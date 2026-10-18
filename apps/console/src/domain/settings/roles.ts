import type { Schemas } from '#/api/api.client'
import { PROVIDER_KINDS, type ProviderKind } from '#/domain/settings/providers'

export type AgentRole = keyof Schemas.ModelAssignmentsDto
export type ModelRef = Schemas.ModelRefDto
export type ModelAssignments = Schemas.ModelAssignmentsDto
export type AssignmentDraft = Partial<Record<AgentRole, ModelRef>>

export interface RoleInfo {
	label: string
	description: string
	capability: keyof Schemas.CapabilitiesResponse
}

export const AGENT_ROLES: ReadonlyArray<AgentRole> = [
	'extractor',
	'judge',
	'writer',
	'embeddings',
]

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
}

export function compatibleModels(
	models: ReadonlyArray<Schemas.ModelResponse>,
	role: AgentRole,
): Array<Schemas.ModelResponse> {
	const capability = ROLE_INFO[role].capability
	return models.filter((model) => model.capabilities[capability])
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
	const { extractor, judge, writer, embeddings } = draft
	if (!extractor || !judge || !writer || !embeddings) return undefined
	return { extractor, judge, writer, embeddings }
}

export function catalogRef(model: Schemas.ModelResponse): ModelRef {
	return { provider: model.provider, model: model.id }
}
