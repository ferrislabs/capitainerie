import { DIRECTION, SKILL_ALL } from '#/domain/agent-team/constants'
import type { TeamAgent } from '#/domain/agent-team/team-agent'

export interface AgentForm {
	name: string
	description: string
	title: string
	role: string
	reports: string
	model: string
	effort: string
	adapter: string
	engine: string
	chrome: boolean
	skip: boolean
	turns: string
	hb: boolean
	interval: string
	instr: string
	gh: string
	trust: string
	pCreate: boolean
	pSkills: boolean
	pAssign: boolean
	canDeleg: boolean
	dScope: string
	dDepth: string
	dPar: string
	dBudget: string
	dReview: string
	dFail: string
}

export type StoreField =
	| 'name'
	| 'description'
	| 'role'
	| 'model'
	| 'reports'
	| 'instr'
export type ConfigFields = Omit<AgentForm, StoreField>

export interface Revision {
	label: string
	author: string
	when: string
	changed: string
}

export interface AgentConfig {
	fields: ConfigFields
	skills: ReadonlyArray<string>
	rev: number
	revs: ReadonlyArray<Revision>
	paused: boolean
}

export const FIELD_LABELS: Readonly<Record<keyof AgentForm, string>> = {
	name: 'nom',
	description: 'description',
	title: 'titre',
	role: 'rôle',
	reports: 'hiérarchie',
	model: 'modèle',
	effort: 'effort de réflexion',
	adapter: 'adaptateur',
	engine: 'moteur',
	chrome: 'Chrome',
	skip: 'permissions',
	turns: 'tours max',
	hb: 'cadence',
	interval: 'intervalle',
	instr: 'instructions',
	gh: 'identité GitHub',
	trust: 'profil de confiance',
	pCreate: 'droit de créer des agents',
	pSkills: "droit d'importer des skills",
	pAssign: "droit d'assigner",
	canDeleg: 'droit de déléguer',
	dScope: 'périmètre de délégation',
	dDepth: 'profondeur de délégation',
	dPar: 'sous-tâches en parallèle',
	dBudget: 'budget par sous-tâche',
	dReview: 'relecture des résultats',
	dFail: 'gestion des échecs',
}

const FORM_KEYS = Object.keys(FIELD_LABELS) as ReadonlyArray<keyof AgentForm>

export function diffForm(
	base: AgentForm,
	next: AgentForm,
): Array<keyof AgentForm> {
	return FORM_KEYS.filter(
		(key) => JSON.stringify(base[key]) !== JSON.stringify(next[key]),
	)
}

export function describeChanges(keys: ReadonlyArray<keyof AgentForm>): string {
	return keys.map((key) => FIELD_LABELS[key]).join(', ')
}

export function buildDefaultConfig(
	agent: Pick<TeamAgent, 'role' | 'model'>,
	overrides: Partial<ConfigFields> & { skills?: ReadonlyArray<string> } = {},
): AgentConfig {
	const { skills, ...fields } = overrides
	return {
		fields: {
			title: `${agent.role} · Natalia`,
			effort: 'Auto',
			adapter: 'Claude Code',
			engine: 'ACP',
			chrome: false,
			skip: false,
			turns: '',
			hb: agent.model !== 'Aucun modèle',
			interval: '1 h',
			gh: 'person',
			trust: 'Collaboration normale',
			pCreate: false,
			pSkills: false,
			pAssign: true,
			canDeleg: false,
			dScope: 'Équipe directe',
			dDepth: '2',
			dPar: '3',
			dBudget: '5 $',
			dReview: "L'agent parent",
			dFail: 'Réessayer 2 fois',
			...fields,
		},
		skills: skills ?? SKILL_ALL.slice(0, 8),
		rev: 0,
		revs: [],
		paused: false,
	}
}

export function buildForm(agent: TeamAgent, config: AgentConfig): AgentForm {
	return {
		...config.fields,
		name: agent.name,
		description: agent.description,
		role: agent.role,
		model: agent.model,
		instr: agent.instructions,
		reports: agent.reports_to ?? DIRECTION,
	}
}
