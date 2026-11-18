import type { Schemas } from '#/api/api.client'
import { slugify } from '#/domain/agent-team/agent-identity'
import type { SevTone } from '#/domain/sources/ui/mockup'

export type Source = Schemas.SourceResponse
export type SourceEvent = Schemas.SourceEventResponse
export type SourceEventStatus = Schemas.SourceEventStatusDto
export type IssuedSource = Schemas.IssuedSourceResponse

export const SLUG_MAX = 60
const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/
export const EVENTS_PAGE_SIZE = 20

export interface SourceDraft {
	name: string
	slug: string
	description: string
	enabled: boolean
	/** Identifiant d'agent, vide : aucun agent par défaut. */
	defaultAgentId: string
	/** Identifiant de projet, vide : aucun projet. */
	projectId: string
	autoRun: boolean
	allowWrites: boolean
}

export const EMPTY_DRAFT: SourceDraft = {
	name: '',
	slug: '',
	description: '',
	enabled: true,
	defaultAgentId: '',
	projectId: '',
	autoRun: false,
	allowWrites: false,
}

export function draftOf(source: Source): SourceDraft {
	return {
		name: source.name,
		slug: source.slug,
		description: source.description,
		enabled: source.enabled,
		defaultAgentId: source.default_agent_id ?? '',
		projectId: source.project_id ?? '',
		autoRun: source.auto_run,
		allowWrites: source.allow_writes,
	}
}

export const slugFromName = (name: string): string =>
	slugify(name).slice(0, SLUG_MAX).replace(/-+$/, '')

/** Message d'erreur du formulaire, ou `null` si le brouillon est valide. */
export function validateDraft(draft: SourceDraft): string | null {
	if (!draft.name.trim()) return 'Le nom est obligatoire.'
	const slug = draft.slug.trim()
	if (!slug) return 'Le slug est obligatoire.'
	if (slug.length > SLUG_MAX)
		return `Le slug ne doit pas dépasser ${SLUG_MAX} caractères.`
	if (!SLUG_PATTERN.test(slug))
		return 'Le slug doit être en kebab-case : minuscules, chiffres et tirets (ex. pieces-chatwoot).'
	if (draft.autoRun && !draft.defaultAgentId)
		return 'Choisis un agent par défaut pour lancer son exécution automatiquement.'
	return null
}

export function createBody(draft: SourceDraft): Schemas.CreateSourceRequest {
	return {
		kind: 'webhook',
		name: draft.name.trim(),
		slug: draft.slug.trim(),
		description: draft.description.trim(),
		enabled: draft.enabled,
		default_agent_id: draft.defaultAgentId || null,
		project_id: draft.projectId || null,
		auto_run: draft.autoRun,
		allow_writes: draft.autoRun && draft.allowWrites,
	}
}

/** Seuls les champs modifiés partent dans le PATCH ; `null` retire l'agent ou le projet. */
export function updateBody(
	source: Source,
	draft: SourceDraft,
): Schemas.UpdateSourceRequest {
	const next = createBody(draft)
	const body: Schemas.UpdateSourceRequest = {}
	if (next.name !== source.name) body.name = next.name
	if (next.slug !== source.slug) body.slug = next.slug
	if (next.description !== source.description)
		body.description = next.description
	if (next.enabled !== source.enabled) body.enabled = next.enabled
	if (next.default_agent_id !== (source.default_agent_id ?? null))
		body.default_agent_id = next.default_agent_id ?? null
	if (next.project_id !== (source.project_id ?? null))
		body.project_id = next.project_id ?? null
	if (next.auto_run !== source.auto_run) body.auto_run = next.auto_run
	if (next.allow_writes !== source.allow_writes)
		body.allow_writes = next.allow_writes
	return body
}

export function filterSources(
	sources: ReadonlyArray<Source>,
	query: string,
): Source[] {
	const needle = query.trim().toLowerCase()
	if (!needle) return [...sources]
	return sources.filter((s) =>
		[s.name, s.slug, s.description].some((field) =>
			field.toLowerCase().includes(needle),
		),
	)
}

export function sourcesOfAgent(
	sources: ReadonlyArray<Source>,
	agentId: string,
): Source[] {
	return sources.filter((s) => s.default_agent_id === agentId)
}

export const EVENT_STATUS: Record<
	SourceEventStatus,
	{ label: string; tone: SevTone }
> = {
	received: { label: 'Reçu', tone: 'info' },
	card_created: { label: 'Tâche créée', tone: 'ok' },
	ignored: { label: 'Ignoré', tone: 'warn' },
	failed: { label: 'Échec', tone: 'crit' },
}

export function describeSourceError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 409) return 'Une source porte déjà ce slug.'
	if (status === 422)
		return 'Source refusée : slug en kebab-case (60 caractères max), nom requis, agent et projet existants.'
	if (status === 403) return "Tu n'as pas le droit de modifier les sources."
	if (status === 404) return "Cette source n'existe plus."
	return "L'opération a échoué. Réessaie."
}
