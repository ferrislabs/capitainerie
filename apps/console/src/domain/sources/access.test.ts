import { beforeEach, describe, expect, it } from 'vitest'
import { demoStore, grantConnector, resetDemoState } from '#/domain/demo'
import {
	crmConnector,
	exportBlocked,
	exportPayload,
	requiredAccess,
	teamAgentIds,
} from '#/domain/sources/access'
import type { Dossier } from '#/domain/sources/criteria'

describe('access', () => {
	beforeEach(resetDemoState)

	it('maps destinations to connectors', () => {
		expect(crmConnector('Odoo (MCP)')).toBe('Odoo CRM (MCP)')
		expect(crmConnector('HubSpot (MCP)')).toBeNull()
	})

	it('includes sub-agents in the team without duplicates', () => {
		const state = demoStore.state
		const team = teamAgentIds(state, ['chief-of-staff'])
		expect(team).toContain('chief-of-staff')
		expect(team.length).toBeGreaterThan(1)
		expect(new Set(team).size).toBe(team.length)
	})

	it('lists who is granted each required connector', () => {
		const rows = requiredAccess(
			demoStore.state,
			['veille-legale'],
			'Odoo (MCP)',
		)
		expect(rows.map((r) => r.connector)).toEqual([
			'RDAP (domaines)',
			'RNE · Annuaire des entreprises',
			'BODACC',
			'Odoo CRM (MCP)',
		])
		expect(rows.every((r) => r.installed)).toBe(true)
		expect(rows.every((r) => r.agentIds.includes('veille-legale'))).toBe(true)
		expect(rows[3]?.step).toBe('Export')
	})

	it('flags uninstalled CRM connectors', () => {
		const rows = requiredAccess(
			demoStore.state,
			['veille-legale'],
			'HubSpot (MCP)',
		)
		expect(rows[3]).toMatchObject({
			connector: 'HubSpot (MCP)',
			installed: false,
			agentIds: [],
		})
	})

	it('reflects a new grant', () => {
		grantConnector('registre', 'RDAP (domaines)')
		const rows = requiredAccess(demoStore.state, ['registre'], 'Odoo (MCP)')
		expect(rows[0]?.agentIds).toEqual(['registre'])
		expect(rows[3]?.agentIds).toEqual([])
	})

	it('blocks export for qualified or review without CRM access', () => {
		expect(exportBlocked('qualified', null, 0)).toBe(true)
		expect(exportBlocked('review', 'Odoo CRM (MCP)', 0)).toBe(true)
		expect(exportBlocked('qualified', 'Odoo CRM (MCP)', 1)).toBe(false)
		expect(exportBlocked('rejected', 'Odoo CRM (MCP)', 0)).toBe(false)
	})

	it('builds a payload only for exportable verdicts', () => {
		const d: Dossier = {
			raison: 'ACME',
			siren: '512 847 391',
			indices: 3,
			active: true,
			collective: false,
			effectif: 150,
			naf: '5229A',
			pays: 'FR',
			crm: false,
			age: 10,
			task: 'NAT-690',
		}
		const q = exportPayload(d, 'iam.acme.fr', true, 'qualified')
		expect(q).toMatchObject({
			model: 'crm.lead',
			mode: 'dry_run',
			name: 'ACME · iam.acme.fr',
			x_siren: '512847391',
			statut_decision: 'qualifié',
		})
		expect(exportPayload(d, 'x', false, 'review')?.mode).toBe('write')
		expect(exportPayload(d, 'x', false, 'rejected')).toBeNull()
	})
})
