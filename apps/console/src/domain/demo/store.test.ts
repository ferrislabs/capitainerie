import { beforeEach, describe, expect, it } from 'vitest'
import {
	agentsWithGrant,
	ancestorIds,
	attachSource,
	demoStore,
	detachSource,
	grantConnector,
	resetDemoState,
	revokeConnector,
	setReportsTo,
	subAgentIds,
} from '#/domain/demo'

describe('demo store', () => {
	beforeEach(resetDemoState)

	it('refuses by default: an agent without grants has none', () => {
		expect(demoStore.state.grants.anouk ?? []).toEqual([])
	})

	it('grants a connector that has an account', () => {
		expect(grantConnector('anouk', 'BODACC')).toBe(true)
		expect(agentsWithGrant(demoStore.state, 'BODACC')).toContain('anouk')
	})

	it('ignores a connector without an account', () => {
		expect(grantConnector('anouk', 'Slack')).toBe(false)
		expect(demoStore.state.grants.anouk ?? []).toEqual([])
	})

	it('does not grant twice and revokes cleanly', () => {
		grantConnector('anouk', 'BODACC')
		grantConnector('anouk', 'BODACC')
		expect(demoStore.state.grants.anouk).toEqual(['BODACC'])
		revokeConnector('anouk', 'BODACC')
		expect(demoStore.state.grants.anouk).toEqual([])
	})

	it('derives sub-agents and ancestors from the reporting line', () => {
		expect(subAgentIds(demoStore.state, 'veille-legale')).toEqual([
			'registre',
			'rdap-dns',
		])
		expect(ancestorIds(demoStore.state, 'registre')).toEqual([
			'veille-legale',
			'chief-of-staff',
		])
	})

	it('moves an agent under another one', () => {
		setReportsTo('anouk', 'Veille légale')
		expect(subAgentIds(demoStore.state, 'veille-legale')).toContain('anouk')
	})

	it('stops an ancestor walk on a cycle', () => {
		setReportsTo('chief-of-staff', 'Registre')
		expect(ancestorIds(demoStore.state, 'registre').length).toBeLessThan(16)
	})

	it('attaches and detaches an agent to a source', () => {
		attachSource('boite-du-cabinet-comptable', 'anouk')
		const attached = demoStore.state.sources.find(
			(s) => s.id === 'boite-du-cabinet-comptable',
		)
		expect(attached?.agentIds).toContain('anouk')
		detachSource('boite-du-cabinet-comptable', 'anouk')
		expect(
			demoStore.state.sources.find((s) => s.id === 'boite-du-cabinet-comptable')
				?.agentIds,
		).not.toContain('anouk')
	})
})
