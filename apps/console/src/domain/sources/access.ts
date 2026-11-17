import { DEMO_CONNECTORS, type DemoState, subAgentIds } from '#/domain/demo'
import type { Dossier, Verdict } from '#/domain/sources/criteria'

export interface AccessRow {
	connector: string
	step: 'Recherche' | 'Export'
	installed: boolean
	agentIds: string[]
}

const CRM_CONNECTORS: Record<string, string> = {
	'Odoo (MCP)': 'Odoo CRM (MCP)',
}

export const SEARCH_CONNECTORS = [
	'RDAP (domaines)',
	'RNE · Annuaire des entreprises',
	'BODACC',
] as const

export function crmConnector(destination: string): string | null {
	return CRM_CONNECTORS[destination] ?? null
}

export function teamAgentIds(
	state: DemoState,
	agentIds: ReadonlyArray<string>,
): string[] {
	return [...new Set(agentIds.flatMap((id) => [id, ...subAgentIds(state, id)]))]
}

export function requiredAccess(
	state: DemoState,
	agentIds: ReadonlyArray<string>,
	destination: string,
): AccessRow[] {
	const team = teamAgentIds(state, agentIds)
	const row = (connector: string, step: AccessRow['step']): AccessRow => ({
		connector,
		step,
		installed: DEMO_CONNECTORS.some((c) => c.name === connector),
		agentIds: team.filter((id) => (state.grants[id] ?? []).includes(connector)),
	})
	return [
		...SEARCH_CONNECTORS.map((c) => row(c, 'Recherche')),
		row(crmConnector(destination) ?? destination, 'Export'),
	]
}

export function exportBlocked(
	verdict: Verdict,
	crm: string | null,
	crmAgentCount: number,
): boolean {
	return verdict !== 'rejected' && (crm === null || crmAgentCount === 0)
}

export function exportPayload(
	dossier: Dossier,
	value: string,
	dry: boolean,
	verdict: Verdict,
): Record<string, string> | null {
	if (verdict === 'rejected') return null
	return {
		model: 'crm.lead',
		mode: dry ? 'dry_run' : 'write',
		name: `${dossier.raison} · ${value}`,
		x_siren: dossier.siren.replace(/ /g, ''),
		description: `Dossier légal : voir ${dossier.task}`,
		x_source_task: dossier.task,
		statut_decision:
			verdict === 'qualified' ? 'qualifié' : 'validé par une personne',
	}
}
