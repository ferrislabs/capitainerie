import type { Schemas } from '#/api/api.client'
import type { SevTone } from '#/domain/audit/ui/sev'

export type AuditEvent = Schemas.AuditEventResponse
export type Severity = Schemas.SeverityDto
export type RunStatus = Schemas.RunStatusResponse
export type Budget = Schemas.BudgetResponse

export interface AgentRef {
	id: string
	name: string
}

export const SEVERITIES: Readonly<
	Record<Severity, { label: string; tone: SevTone }>
> = {
	critical: { label: 'Critique', tone: 'crit' },
	warning: { label: 'Avertissement', tone: 'warn' },
	info: { label: 'Information', tone: 'info' },
	success: { label: 'Succès', tone: 'ok' },
	created: { label: 'Création', tone: 'vio' },
}

export const RUN_STATUSES: ReadonlyArray<{
	status: RunStatus
	label: string
	tone: SevTone
}> = [
	{ status: 'queued', label: 'en file', tone: 'info' },
	{ status: 'running', label: 'en cours', tone: 'vio' },
	{ status: 'succeeded', label: 'terminée', tone: 'ok' },
	{ status: 'failed', label: 'échec', tone: 'crit' },
	{ status: 'cancelled', label: 'annulée', tone: 'warn' },
]

export function runStatusOf(status: RunStatus) {
	return (
		RUN_STATUSES.find((item) => item.status === status) ?? {
			status,
			label: status,
			tone: 'info' as const,
		}
	)
}

export function runAgent(run: Schemas.RunResponse): AgentRef | null {
	return run.agent ? { id: run.agent.id, name: run.agent.name } : null
}

export function runDuration(
	run: Schemas.RunResponse,
	now: number = Date.now(),
): string {
	if (!run.started_at) return '–'
	const end = run.finished_at ? Date.parse(run.finished_at) : now
	const seconds = Math.max(
		0,
		Math.round((end - Date.parse(run.started_at)) / 1000),
	)
	if (Number.isNaN(seconds)) return '–'
	if (seconds < 60) return `${seconds} s`
	const minutes = Math.floor(seconds / 60)
	if (minutes < 60)
		return `${minutes} min ${String(seconds % 60).padStart(2, '0')}`
	return `${Math.floor(minutes / 60)} h ${String(minutes % 60).padStart(2, '0')}`
}

export interface MonthWindow {
	from: string
	to: string
}

export function monthWindow(now: Date): MonthWindow {
	const year = now.getUTCFullYear()
	const month = now.getUTCMonth()
	return {
		from: new Date(Date.UTC(year, month, 1)).toISOString(),
		to: new Date(Date.UTC(year, month + 1, 1)).toISOString(),
	}
}

export function initials(name: string): string {
	const words = name.trim().split(/\s+/).filter(Boolean)
	const letters = words.length > 1 ? [words[0], words.at(-1)] : [name.trim()]
	return letters
		.map((word) => (word ?? '').slice(0, words.length > 1 ? 1 : 2))
		.join('')
		.toUpperCase()
}

const TIME = new Intl.DateTimeFormat('fr-FR', {
	hour: '2-digit',
	minute: '2-digit',
})
const DAY = new Intl.DateTimeFormat('fr-FR', {
	weekday: 'long',
	day: 'numeric',
	month: 'long',
})

export const timeOf = (iso: string): string => TIME.format(new Date(iso))

export interface DayGroup {
	day: string
	events: ReadonlyArray<AuditEvent>
}

/** Regroupe des événements déjà triés par jour local, dans l'ordre reçu. */
export function groupByDay(events: ReadonlyArray<AuditEvent>): DayGroup[] {
	const groups: { day: string; events: AuditEvent[] }[] = []
	for (const event of events) {
		const day = DAY.format(new Date(event.occurred_at))
		const last = groups.at(-1)
		if (last?.day === day) last.events.push(event)
		else groups.push({ day, events: [event] })
	}
	return groups
}

const MICROS_PER_CENT = 10_000

export const microsToCents = (micros: number): number =>
	Math.round(micros / MICROS_PER_CENT)

const USD = new Intl.NumberFormat('fr-FR', {
	style: 'currency',
	currency: 'USD',
	currencyDisplay: 'narrowSymbol',
})

export const formatCents = (cents: number): string =>
	USD.format(cents / 100).replace(/ /g, ' ')

export interface BudgetDraft {
	/** `null` pour le budget de l'organisation. */
	agentId: string | null
	label: string
	limit: string
	threshold: string
}

export function draftsOf(budgets: ReadonlyArray<Budget>): BudgetDraft[] {
	return budgets.map((budget) => ({
		agentId: budget.scope === 'organization' ? null : (budget.agent_id ?? null),
		label:
			budget.scope === 'organization'
				? 'Organisation'
				: (budget.agent_name ?? 'Agent'),
		limit: String(budget.monthly_limit_cents / 100),
		threshold: String(budget.alert_threshold_percent),
	}))
}

export type DraftResult =
	| { ok: true; request: Schemas.SaveBudgetsRequest }
	| { ok: false; reason: string }

function parseLimit(value: string): number | null {
	const amount = Number(value.replace(',', '.').trim())
	if (!value.trim() || !Number.isFinite(amount) || amount < 0) return null
	return Math.round(amount * 100)
}

function parseThreshold(value: string): number | null | undefined {
	if (!value.trim()) return undefined
	const percent = Number(value.trim())
	return Number.isInteger(percent) && percent >= 1 && percent <= 100
		? percent
		: null
}

/** Construit la requête `PUT /budgets`, qui remplace l'ensemble des budgets. */
export function saveRequestOf(drafts: ReadonlyArray<BudgetDraft>): DraftResult {
	let organization: Schemas.LimitRequest | null = null
	const agents: Schemas.AgentLimitRequest[] = []
	for (const draft of drafts) {
		const cents = parseLimit(draft.limit)
		if (cents === null)
			return { ok: false, reason: `Montant invalide pour ${draft.label}.` }
		const threshold = parseThreshold(draft.threshold)
		if (threshold === null)
			return {
				ok: false,
				reason: `Seuil d'alerte invalide pour ${draft.label} (1 à 100 %).`,
			}
		const limit: Schemas.LimitRequest = {
			monthly_limit_cents: cents,
			...(threshold === undefined
				? {}
				: { alert_threshold_percent: threshold }),
		}
		if (draft.agentId === null) organization = limit
		else agents.push({ agent_id: draft.agentId, ...limit })
	}
	return { ok: true, request: { organization, agents } }
}

export function describeBudgetError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 403)
		return 'Seuls les administrateurs de la Capitainerie peuvent modifier les budgets.'
	if (status === 422)
		return 'Budgets refusés par le serveur : vérifie les montants.'
	return "Les budgets n'ont pas pu être enregistrés."
}
