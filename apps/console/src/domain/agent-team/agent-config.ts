import { DIRECTION, SKILL_ALL } from '#/domain/agent-team/constants'
import {
	DEFAULT_KEYS,
	DEFAULT_REVISIONS,
	defaultInstructions,
} from '#/domain/agent-team/mock/agent-fixtures'
import type { DemoAgent } from '#/domain/demo'

export interface EnvVar {
	id: string
	name: string
	secret: string
	version: string
}

export interface AgentForm {
	name: string
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
	env: ReadonlyArray<EnvVar>
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

export type StoreField = 'name' | 'role' | 'model' | 'reports'
export type ConfigFields = Omit<AgentForm, StoreField>

export interface Revision {
	label: string
	author: string
	when: string
	changed: string
}

export interface ApiKey {
	name: string
	preview: string
	created: string
	lastUsed: string
}

export interface AgentConfig {
	fields: ConfigFields
	skills: ReadonlyArray<string>
	rev: number
	revs: ReadonlyArray<Revision>
	keys: ReadonlyArray<ApiKey>
	paused: boolean
}

export const FIELD_LABELS: Readonly<Record<keyof AgentForm, string>> = {
	name: 'nom',
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
	env: 'variables',
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
	agent: DemoAgent,
	overrides: Partial<ConfigFields> & { skills?: ReadonlyArray<string> } = {},
): AgentConfig {
	const { skills, ...fields } = overrides
	const delegates =
		agent.id === 'chief-of-staff' || agent.id === 'veille-legale'
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
			instr: defaultInstructions(agent),
			env: [
				{
					id: 'gitlab-token',
					name: 'GITLAB_TOKEN',
					secret: 'gitlab-com-natalia',
					version: 'latest',
				},
			],
			gh: 'person',
			trust: 'Collaboration normale',
			pCreate: agent.id === 'chief-of-staff',
			pSkills: false,
			pAssign: true,
			canDeleg: delegates,
			dScope: 'Équipe directe',
			dDepth: '2',
			dPar: '3',
			dBudget: '5 $',
			dReview: "L'agent parent",
			dFail: 'Réessayer 2 fois',
			...fields,
		},
		skills: skills ?? SKILL_ALL.slice(0, 8),
		rev: 14,
		revs: DEFAULT_REVISIONS.map((r) => ({ ...r })),
		keys: DEFAULT_KEYS.map((k) => ({ ...k })),
		paused: false,
	}
}

export function buildForm(
	agent: DemoAgent,
	reports: string | null,
	config: AgentConfig,
): AgentForm {
	return {
		...config.fields,
		name: agent.name,
		role: agent.role,
		model: agent.model,
		reports: reports ?? DIRECTION,
	}
}
