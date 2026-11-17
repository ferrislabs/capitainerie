import { beforeEach, describe, expect, it } from 'vitest'
import {
	addAgent,
	addApproval,
	addTask,
	agentsWithGrant,
	ancestorIds,
	attachSource,
	childTasks,
	decideApproval,
	demoStore,
	detachSource,
	grantConnector,
	nextTaskId,
	resetDemoState,
	revokeConnector,
	setReportsTo,
	subAgentIds,
	taskDepth,
	updateTask,
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

	it('lists an agent created during the session with its reporting line', () => {
		addAgent({
			id: 'nouvel-agent',
			name: 'Nouvel agent',
			role: 'Chercheur',
			description: '',
			status: 'idle',
			hue: 10,
			model: 'Claude Sonnet 5.5',
			reportsTo: 'Veille légale',
		})
		expect(subAgentIds(demoStore.state, 'veille-legale')).toContain(
			'nouvel-agent',
		)
		expect(demoStore.state.agents.at(-1)?.id).toBe('nouvel-agent')
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

	it('creates the next task id above the highest existing one', () => {
		expect(nextTaskId(demoStore.state)).toBe('NAT-696')
		addTask({
			id: 'NAT-696',
			title: 'Test',
			status: 'todo',
			agentId: 'anouk',
			age: "à l'instant",
			project: '',
		})
		expect(nextTaskId(demoStore.state)).toBe('NAT-697')
	})

	it('derives children and depth from the parent link', () => {
		expect(childTasks(demoStore.state, 'NAT-691').map((t) => t.id)).toEqual([
			'NAT-695',
			'NAT-694',
		])
		expect(taskDepth(demoStore.state, 'NAT-695')).toBe(1)
		expect(taskDepth(demoStore.state, 'NAT-691')).toBe(0)
	})

	it('updates a task in place', () => {
		updateTask('NAT-695', { status: 'done' })
		expect(demoStore.state.tasks.find((t) => t.id === 'NAT-695')?.status).toBe(
			'done',
		)
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
