import type { Schemas } from '#/api/api.client'

export type Harness = Schemas.HarnessResponse
export type ProviderKind = Schemas.ProviderKindDto

export interface HarnessChip {
	id: Schemas.HarnessDto
	name: string
	initial: string
	hue: number
}

export const HARNESS_CHIPS: ReadonlyArray<HarnessChip> = [
	{ id: 'claude-code', name: 'Claude Code', initial: 'C', hue: 18 },
	{ id: 'codex', name: 'Codex', initial: 'X', hue: 235 },
	{ id: 'mistral-vibe', name: 'Mistral Vibe', initial: 'M', hue: 24 },
	{ id: 'antigravity', name: 'Antigravity', initial: 'G', hue: 212 },
]

export const DEFAULT_ADAPTER = 'Claude Code'

export const PROVIDER_NAMES: Readonly<Record<ProviderKind, string>> = {
	anthropic: 'Anthropic',
	openai: 'OpenAI',
	mistral: 'Mistral',
	google: 'Google',
	openai_compatible: 'OpenAI compatible',
}

export const KEY_PLACEHOLDERS: Readonly<Record<ProviderKind, string>> = {
	anthropic: 'sk-ant-…',
	openai: 'sk-…',
	mistral: '',
	google: 'AIza…',
	openai_compatible: 'sk-…',
}

export function chipOf(value: string): HarnessChip | undefined {
	return HARNESS_CHIPS.find((c) => c.id === value || c.name === value)
}

export function harnessOfModel(
	harnesses: ReadonlyArray<Harness>,
	model: string,
): Harness | undefined {
	const exact = harnesses.find((h) => h.models.includes(model))
	if (exact) return exact
	if (model.startsWith('claude-')) {
		return harnesses.find((h) => h.id === 'claude-code')
	}
	return undefined
}

export function harnessOfAgent(
	harnesses: ReadonlyArray<Harness>,
	model: string,
	configAdapter: string,
): Harness | undefined {
	return (
		harnessOfModel(harnesses, model) ??
		harnesses.find((h) => h.name === configAdapter) ??
		harnesses.find((h) => h.id === 'claude-code')
	)
}

export function adapterNameOf(
	harnesses: ReadonlyArray<Harness>,
	model: string,
	configAdapter: string,
): string {
	const found = harnessOfModel(harnesses, model)
	if (found) return found.name
	return chipOf(configAdapter)?.name ?? DEFAULT_ADAPTER
}

export type KeySource =
	| { kind: 'loading' }
	| { kind: 'personal'; preview: string }
	| { kind: 'organization' }
	| { kind: 'organization-unknown' }
	| { kind: 'none' }

export function keySourceOf(
	kind: ProviderKind,
	mine: ReadonlyArray<Schemas.UserProviderResponse> | undefined,
	org: ReadonlyArray<Schemas.ProviderResponse> | 'forbidden' | undefined,
): KeySource {
	if (mine === undefined || org === undefined) return { kind: 'loading' }
	const personal = mine.find((p) => p.kind === kind)
	if (personal) return { kind: 'personal', preview: personal.preview }
	if (org === 'forbidden') return { kind: 'organization-unknown' }
	return org.some((p) => p.kind === kind)
		? { kind: 'organization' }
		: { kind: 'none' }
}

export function keySourceLabel(source: KeySource, typed: boolean): string {
	if (typed) return 'Ta nouvelle clé personnelle'
	switch (source.kind) {
		case 'personal':
			return `Ta clé personnelle (${source.preview})`
		case 'organization':
			return "Clé de l'organisation"
		case 'organization-unknown':
			return "Clé de l'organisation, si elle existe"
		default:
			return 'Aucune clé'
	}
}

export function hasUsableKey(source: KeySource): boolean {
	return source.kind !== 'loading' && source.kind !== 'none'
}
