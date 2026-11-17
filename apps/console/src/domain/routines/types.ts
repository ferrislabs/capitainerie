import type { Schedule } from '#/domain/routines/schedule'

export const PRIORITIES = ['Basse', 'Moyenne', 'Haute', 'Critique'] as const
export const IF_OPEN_POLICIES = [
	'Ignorer si déjà ouverte',
	'Mettre en file',
	'Créer quand même',
] as const
export const IF_MISSED_POLICIES = [
	'Ignorer',
	'Rattraper une seule fois',
	'Rattraper tout',
] as const
export const TIMEZONES = ['Europe/Paris', 'UTC', 'America/New_York'] as const
export const NO_PROJECT = 'Aucun'

export type RunTone = 'success' | 'danger' | 'neutral' | 'violet'
export type RunOrigin = 'Planning' | 'Manuel' | 'Webhook'

export interface RunOutcome {
	when: string
	tone: RunTone
	label: string
}

export interface RoutineRun extends RunOutcome {
	id: string
	origin: RunOrigin
	duration: string
	taskId: string
}

export interface PlanningTrigger {
	kind: 'planning'
	id: string
	enabled: boolean
	schedule: Schedule
}

export interface WebhookTrigger {
	kind: 'webhook'
	id: string
	enabled: boolean
	url: string
	secret: string
}

export type Trigger = PlanningTrigger | WebhookTrigger

export interface RoutineSettings {
	name: string
	instruction: string
	agent: string
	project: string
	priority: string
	titleTemplate: string
	ifOpen: string
	ifMissed: string
	timezone: string
}

export interface Routine extends RoutineSettings {
	id: string
	enabled: boolean
	lastRun: RunOutcome
	listSchedule?: string | undefined
	triggers: ReadonlyArray<Trigger>
	runs: ReadonlyArray<RoutineRun>
}
