import type { Schemas } from '#/api/api.client'
import {
	type AgentConfig,
	buildDefaultConfig,
} from '#/domain/agent-team/agent-config'
import { hueOf } from '#/domain/agent-team/agent-identity'
import { DIRECTION } from '#/domain/agent-team/constants'
import { isValidModelId } from '#/domain/agent-team/model-id'
import type { TeamAgent } from '#/domain/agent-team/team-agent'

export type Connection = 'sub' | 'key' | 'gw'

export interface WizardState {
	step: number
	name: string
	adapter: string
	conn: Connection
	key: string
	role: string
	title: string
	reports: string
	model: string
	effort: string
	interval: string
	instr: string
	skills: ReadonlyArray<string>
	sources: ReadonlyArray<string>
	connectors: ReadonlyArray<string>
	tested: boolean
}

export const WIZARD_LAST_STEP = 4

export function initialWizard(): WizardState {
	return {
		step: 0,
		name: '',
		adapter: '',
		conn: 'sub',
		key: '',
		role: 'Général',
		title: '',
		reports: DIRECTION,
		model: '',
		effort: 'Auto',
		interval: '1 h',
		instr: '',
		skills: ['paperclip', 'para-memory-files'],
		sources: [],
		connectors: [],
		tested: false,
	}
}

export function canAdvance(w: WizardState): boolean {
	if (w.step === 0) return w.name.trim().length > 0
	if (w.step === 1) return w.adapter.length > 0
	if (w.step === 3 && !isValidModelId(w.model)) return false
	if (w.step === 2 && w.conn !== 'sub') return w.key.trim().length > 0
	return true
}

export function toggled<T>(list: ReadonlyArray<T>, item: T): T[] {
	return list.includes(item) ? list.filter((x) => x !== item) : [...list, item]
}

export function buildCreateRequest(
	w: WizardState,
	slug: string,
	grantable: ReadonlyArray<string>,
): Schemas.CreateTeamAgentRequest {
	const name = w.name.trim()
	const instructions = w.instr.trim()
	return {
		slug,
		name,
		role: w.role,
		description: w.title.trim(),
		model: w.model,
		hue: hueOf(name),
		reports_to: w.reports === DIRECTION ? null : w.reports,
		connector_grants: w.connectors.filter((c) => grantable.includes(c)),
		...(instructions ? { instructions } : {}),
	}
}

export function buildWizardConfig(
	w: WizardState,
	agent: TeamAgent,
): AgentConfig {
	const base = buildDefaultConfig(agent, {
		title: w.title || `${w.role} · Natalia`,
		adapter: w.adapter,
		effort: w.effort,
		hb: w.interval !== 'Manuelle',
		interval: w.interval === 'Manuelle' ? '1 h' : w.interval,
		canDeleg: false,
		pCreate: false,
		skills: w.skills,
		...(w.instr.trim() ? { instr: w.instr } : {}),
	})
	return {
		...base,
		rev: 1,
		revs: [
			{
				label: 'Rév. 1',
				author: 'Baptiste Parmantier',
				when: "à l'instant",
				changed: 'création',
			},
		],
	}
}
