import type { Schemas } from '#/api/api.client'
import { buildDefaultConfig } from '#/domain/agent-team/agent-config'
import { hueOf } from '#/domain/agent-team/agent-identity'
import { DIRECTION } from '#/domain/agent-team/constants'
import { isValidModelId } from '#/domain/agent-team/model-id'
import type { TeamAgent } from '#/domain/agent-team/team-agent'

export interface WizardState {
	step: number
	name: string
	adapter: string
	key: string
	role: string
	title: string
	reports: string
	model: string
	effort: string
	interval: string
	instr: string
	skills: ReadonlyArray<string>
	connectors: ReadonlyArray<string>
	repositories: ReadonlyArray<string>
}

export const WIZARD_LAST_STEP = 4

export function initialWizard(): WizardState {
	return {
		step: 0,
		name: '',
		adapter: '',
		key: '',
		role: 'Général',
		title: '',
		reports: DIRECTION,
		model: '',
		effort: 'Auto',
		interval: '1 h',
		instr: '',
		skills: [],
		connectors: [],
		repositories: [],
	}
}

export function canAdvance(w: WizardState, keyAvailable = true): boolean {
	if (w.step === 0) return w.name.trim().length > 0
	if (w.step === 1) return w.adapter.length > 0
	if (w.step === 2) return keyAvailable || w.key.trim().length > 0
	if (w.step === 3) return w.model.length > 0 && isValidModelId(w.model)
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
		...(w.repositories.length
			? { repository_grants: [...w.repositories] }
			: {}),
		...(instructions ? { instructions } : {}),
	}
}

export function buildWizardPatch(
	w: WizardState,
	agent: TeamAgent,
	adapterName: string = w.adapter,
): Schemas.UpdateTeamAgentRequest {
	return {
		config: {
			...agent.config,
			...buildDefaultConfig(agent, {
				title: w.title.trim() || w.role,
				adapter: adapterName,
				effort: w.effort,
				hb: w.interval !== 'Manuelle',
				interval: w.interval === 'Manuelle' ? '1 h' : w.interval,
			}),
		},
		skills: [...w.skills],
		change_summary: 'configuration initiale',
	}
}
