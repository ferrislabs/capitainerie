import type { Schemas } from '#/api/api.client'
import { slugify } from '#/domain/agent-team/agent-identity'
import { relativeAge } from '#/domain/dashboard/format'
import {
	defaultSchedule,
	type ScheduleDraft,
	scheduleDto,
	scheduleError,
	scheduleText,
} from '#/domain/routines/schedule'
import type { SevTone } from '#/domain/sources/ui/kit'
import { PRIORITY_LABEL } from '#/domain/tasks/model'

export type Routine = Schemas.RoutineResponse
export type RoutineRun = Schemas.RoutineRunResponse
export type Trigger = Schemas.TriggerResponse
export type IssuedTrigger = Schemas.IssuedTriggerResponse
export type IfOpen = Schemas.IfOpenDto
export type IfMissed = Schemas.IfMissedDto
export type Priority = Schemas.PriorityDto

export const SLUG_MAX = 60
const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/
export const NAME_MAX = 120
export const TITLE_TEMPLATE_MAX = 200
export const RUNS_PAGE_SIZE = 20
export const DEFAULT_TITLE_TEMPLATE = '{nom} · {date}'
export const DEFAULT_TIMEZONE = 'Europe/Paris'

export interface PolicyOption<T extends string> {
	value: T
	label: string
	help: string
}

export const IF_OPEN_OPTIONS: ReadonlyArray<PolicyOption<IfOpen>> = [
	{
		value: 'skip',
		label: 'Ignorer ce passage',
		help: "Si la tâche du passage précédent n'est pas terminée, aucune nouvelle tâche n'est créée : le passage est noté « ignoré ».",
	},
	{
		value: 'queue',
		label: 'Attendre la fin de la précédente',
		help: 'Le passage attend que la tâche précédente soit terminée, puis crée la sienne. Un seul passage attend à la fois.',
	},
	{
		value: 'always',
		label: 'Créer quand même',
		help: 'Une nouvelle tâche est créée à chaque passage, même si les précédentes sont encore ouvertes.',
	},
]

export const IF_MISSED_OPTIONS: ReadonlyArray<PolicyOption<IfMissed>> = [
	{
		value: 'skip',
		label: 'Ignorer les passages manqués',
		help: 'Un passage en retard de plus de 10 minutes (serveur arrêté, par exemple) est noté « ignoré » ; la routine reprend au passage suivant.',
	},
	{
		value: 'run_once',
		label: 'Rattraper une seule fois',
		help: 'Au redémarrage, un seul passage est lancé pour tous ceux qui ont été manqués.',
	},
	{
		value: 'run_all',
		label: 'Rattraper chaque passage',
		help: 'Au redémarrage, chaque passage manqué crée sa propre tâche, dans l’ordre.',
	},
]

export const policyLabel = <T extends string>(
	options: ReadonlyArray<PolicyOption<T>>,
	value: T,
): string => options.find((o) => o.value === value)?.label ?? value

export const PRIORITY_OPTIONS: ReadonlyArray<{
	value: Priority
	label: string
}> = (['low', 'normal', 'high', 'urgent'] as const).map((value) => ({
	value,
	label: PRIORITY_LABEL[value],
}))

export const ORIGIN_LABEL: Record<Schemas.RunOriginDto, string> = {
	planning: 'Planning',
	manual: 'Manuel',
	webhook: 'Webhook',
}

const FALLBACK_TIMEZONES = [
	'Europe/Paris',
	'Europe/London',
	'Europe/Berlin',
	'Europe/Brussels',
	'Europe/Zurich',
	'America/New_York',
	'America/Montreal',
	'America/Los_Angeles',
	'Asia/Tokyo',
	'UTC',
]

/** Fuseaux IANA du navigateur, avec `current` toujours présent. */
export function timezoneOptions(current: string): ReadonlyArray<string> {
	const intl = Intl as { supportedValuesOf?: (key: string) => string[] }
	let zones: ReadonlyArray<string> = FALLBACK_TIMEZONES
	try {
		zones = intl.supportedValuesOf?.('timeZone') ?? FALLBACK_TIMEZONES
	} catch {
		zones = FALLBACK_TIMEZONES
	}
	return [...new Set([DEFAULT_TIMEZONE, 'UTC', current, ...zones])]
		.filter(Boolean)
		.sort((a, b) => a.localeCompare(b))
}

export interface RoutineDraft {
	name: string
	slug: string
	prompt: string
	/** Identifiant de l'agent, vide tant qu'aucun n'est choisi. */
	agentId: string
	/** Identifiant de projet, vide : aucun projet. */
	projectId: string
	priority: Priority
	titleTemplate: string
	timezone: string
	enabled: boolean
	ifOpen: IfOpen
	ifMissed: IfMissed
	allowWrites: boolean
	/** Création seulement : ajoute un déclencheur planning. */
	planned: boolean
	schedule: ScheduleDraft
}

export const emptyDraft = (): RoutineDraft => ({
	name: '',
	slug: '',
	prompt: '',
	agentId: '',
	projectId: '',
	priority: 'normal',
	titleTemplate: DEFAULT_TITLE_TEMPLATE,
	timezone: DEFAULT_TIMEZONE,
	enabled: true,
	ifOpen: 'skip',
	ifMissed: 'run_once',
	allowWrites: false,
	planned: true,
	schedule: defaultSchedule(),
})

export function draftOf(routine: Routine): RoutineDraft {
	return {
		...emptyDraft(),
		name: routine.name,
		slug: routine.slug,
		prompt: routine.prompt,
		agentId: routine.agent_id,
		projectId: routine.project_id ?? '',
		priority: routine.priority,
		titleTemplate: routine.title_template,
		timezone: routine.timezone,
		enabled: routine.enabled,
		ifOpen: routine.if_open,
		ifMissed: routine.if_missed,
		allowWrites: routine.allow_writes,
		planned: false,
	}
}

export const slugFromName = (name: string): string =>
	slugify(name).slice(0, SLUG_MAX).replace(/-+$/, '')

/** Message d'erreur du formulaire, ou `null` si le brouillon est valide. */
export function validateDraft(draft: RoutineDraft): string | null {
	const name = draft.name.trim()
	if (!name) return 'Donne un nom à la routine.'
	if (name.length > NAME_MAX)
		return `Le nom ne doit pas dépasser ${NAME_MAX} caractères.`
	const slug = draft.slug.trim()
	if (slug && (slug.length > SLUG_MAX || !SLUG_PATTERN.test(slug)))
		return `Le slug doit être en kebab-case (minuscules, chiffres et tirets, ${SLUG_MAX} caractères max).`
	if (!draft.prompt.trim()) return 'Écris la consigne de la routine.'
	if (!draft.agentId) return "Choisis l'agent qui reçoit les tâches."
	if (draft.titleTemplate.trim().length > TITLE_TEMPLATE_MAX)
		return `Le titre ne doit pas dépasser ${TITLE_TEMPLATE_MAX} caractères.`
	if (!draft.timezone) return 'Choisis un fuseau horaire.'
	return draft.planned ? scheduleError(draft.schedule) : null
}

export function createBody(draft: RoutineDraft): Schemas.CreateRoutineRequest {
	const slug = draft.slug.trim()
	return {
		name: draft.name.trim(),
		...(slug ? { slug } : {}),
		prompt: draft.prompt.trim(),
		agent_id: draft.agentId,
		project_id: draft.projectId || null,
		priority: draft.priority,
		title_template: draft.titleTemplate.trim() || DEFAULT_TITLE_TEMPLATE,
		timezone: draft.timezone,
		enabled: draft.enabled,
		if_open: draft.ifOpen,
		if_missed: draft.ifMissed,
		allow_writes: draft.allowWrites,
		schedule: draft.planned ? scheduleDto(draft.schedule) : null,
	}
}

/** Seuls les champs modifiés partent dans le PATCH ; `null` retire le projet. */
export function updateBody(
	routine: Routine,
	draft: RoutineDraft,
): Schemas.UpdateRoutineRequest {
	const next = createBody({ ...draft, planned: false })
	const body: Schemas.UpdateRoutineRequest = {}
	if (next.name !== routine.name) body.name = next.name
	if (next.slug !== undefined && next.slug !== routine.slug)
		body.slug = next.slug
	if (next.prompt !== routine.prompt) body.prompt = next.prompt
	if (next.agent_id !== routine.agent_id) body.agent_id = next.agent_id
	if ((next.project_id ?? null) !== (routine.project_id ?? null))
		body.project_id = next.project_id ?? null
	if (next.priority !== routine.priority) body.priority = next.priority
	if (next.title_template !== routine.title_template)
		body.title_template = next.title_template
	if (next.timezone !== routine.timezone) body.timezone = next.timezone
	if (next.enabled !== routine.enabled) body.enabled = next.enabled
	if (next.if_open !== routine.if_open) body.if_open = next.if_open
	if (next.if_missed !== routine.if_missed) body.if_missed = next.if_missed
	if (next.allow_writes !== routine.allow_writes)
		body.allow_writes = next.allow_writes
	return body
}

export function filterRoutines(
	routines: ReadonlyArray<Routine>,
	query: string,
	agentName: (id: string) => string | undefined = () => undefined,
): Routine[] {
	const needle = query.trim().toLowerCase()
	if (!needle) return [...routines]
	return routines.filter((r) =>
		[r.name, r.slug, r.prompt, agentName(r.agent_id) ?? ''].some((field) =>
			field.toLowerCase().includes(needle),
		),
	)
}

/** Résumé des déclencheurs : plannings d'abord, puis webhook, sinon lancement manuel. */
export function triggersSummary(routine: Routine): string {
	const parts = routine.triggers.map((t) =>
		t.kind === 'planning' && t.schedule ? scheduleText(t.schedule) : 'Webhook',
	)
	return parts.length > 0 ? parts.join(' · ') : 'Manuelle'
}

export interface RunOutcome {
	label: string
	tone: SevTone
}

export function runOutcome(run: RoutineRun): RunOutcome {
	switch (run.status) {
		case 'created':
			if (run.card_column === 'done') return { label: 'terminée', tone: 'ok' }
			if (run.card_column === 'rejected')
				return { label: 'rejetée', tone: 'info' }
			if (run.run_status === 'failed') return { label: 'échec', tone: 'crit' }
			return { label: 'tâche créée', tone: 'vio' }
		case 'skipped':
			return { label: 'ignorée', tone: 'warn' }
		case 'failed':
			return { label: 'échec', tone: 'crit' }
		case 'pending':
		case 'queued':
			return { label: 'en attente', tone: 'info' }
	}
}

function formatDuration(ms: number): string {
	const seconds = Math.max(0, Math.round(ms / 1000))
	if (seconds < 60) return `${seconds} s`
	const minutes = Math.floor(seconds / 60)
	if (minutes < 60) {
		const rest = seconds % 60
		return rest > 0
			? `${minutes} min ${String(rest).padStart(2, '0')}`
			: `${minutes} min`
	}
	const hours = Math.floor(minutes / 60)
	const rest = minutes % 60
	return rest > 0 ? `${hours} h ${String(rest).padStart(2, '0')}` : `${hours} h`
}

/** Durée de l'exécution de l'agent : « en cours » tant qu'elle n'est pas finie. */
export function runDuration(run: RoutineRun): string {
	if (!run.run_started_at) return '—'
	if (!run.run_finished_at) return 'en cours'
	const ms = Date.parse(run.run_finished_at) - Date.parse(run.run_started_at)
	return Number.isNaN(ms) ? '—' : formatDuration(ms)
}

/** « dans 3 h » pour une date future, « il y a 2 j » pour une date passée. */
export function relativeTime(iso: string, now: number = Date.now()): string {
	const delta = Date.parse(iso) - now
	if (Number.isNaN(delta)) return ''
	if (delta <= 0) return relativeAge(iso, now)
	const minutes = Math.round(delta / 60_000)
	if (minutes < 1) return "dans moins d'une minute"
	if (minutes < 60) return `dans ${minutes} min`
	const hours = Math.round(minutes / 60)
	if (hours < 24) return `dans ${hours} h`
	return `dans ${Math.round(hours / 24)} j`
}

export const formatDateTime = (iso: string): string =>
	new Date(iso).toLocaleString('fr-FR', {
		weekday: 'short',
		day: 'numeric',
		month: 'short',
		hour: '2-digit',
		minute: '2-digit',
	})

export function describeRoutineError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	const message = (error as { message?: string } | null)?.message
	if (status === 409) return 'Une routine porte déjà ce slug.'
	if (status === 422)
		return `Routine refusée${message ? ` : ${message}` : '.'} Vérifie le slug, le fuseau, le planning, l'agent et le projet.`
	if (status === 403) return "Tu n'as pas le droit de modifier cette routine."
	if (status === 404) return "Cette routine n'existe plus."
	return "L'opération a échoué. Réessaie."
}
