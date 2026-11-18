import { beforeEach, describe, expect, it } from 'vitest'
import {
	addApproval,
	attachSource,
	decideApproval,
	demoStore,
	detachSource,
	grantConnector,
	resetDemoState,
	revokeConnector,
	subAgentIds,
} from '#/domain/demo'

describe('demo store', () => {
	beforeEach(resetDemoState)

	it('refuses by default: an agent without grants has none', () => {
		expect(demoStore.state.grants.anouk ?? []).toEqual([])
	})

	it('grants a connector that has an account', () => {
		expect(grantConnector('anouk', 'BODACC')).toBe(true)
		expect(demoStore.state.grants.anouk).toContain('BODACC')
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

	it('derives sub-agents from the reporting line', () => {
		expect(subAgentIds(demoStore.state, 'veille-legale')).toEqual([
			'registre',
			'rdap-dns',
		])
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

	it('decides a pending approval once, and only once', () => {
		addApproval({
			id: 'APP-9',
			title: 'À valider',
			detail: '',
			requestedBy: null,
			status: 'pending',
			at: "à l'instant",
			decidedBy: '',
		})
		decideApproval('APP-9', 'approved', 'Board')
		decideApproval('APP-9', 'rejected', 'Autre')
		const decided = demoStore.state.approvals.find((a) => a.id === 'APP-9')
		expect(decided?.status).toBe('approved')
		expect(decided?.decidedBy).toBe('Board')
	})
})
