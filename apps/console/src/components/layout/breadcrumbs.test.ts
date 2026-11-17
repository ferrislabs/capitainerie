import { describe, expect, it } from 'vitest'
import { breadcrumbs, type EntityNames } from '#/components/layout/breadcrumbs'

const NAMES: EntityNames = {
	task: (id) => (id === 'NAT-611' ? 'Revue sécurité quotidienne' : undefined),
	agent: (id) => (id === 'dev' ? 'Dev (cycle autonome)' : undefined),
	source: (id) => (id === 'dns' ? 'Nouveaux sous-domaines' : undefined),
	routine: (id) => (id === 'analyse' ? 'Analyse board' : undefined),
}

describe('breadcrumbs', () => {
	it('names the dashboard on the home page', () => {
		expect(breadcrumbs('/')).toEqual([{ label: 'Tableau de bord' }])
	})

	it('shows a section page alone, tabs included', () => {
		expect(breadcrumbs('/tasks')).toEqual([{ label: 'Tâches' }])
		expect(breadcrumbs('/audit/runs')).toEqual([{ label: 'Audit' }])
		expect(breadcrumbs('/agents/organigramme')).toEqual([{ label: 'Agents' }])
	})

	it('names a task with its id tag', () => {
		expect(breadcrumbs('/tasks/nat-611', NAMES)).toEqual([
			{ label: 'Tâches', to: '/tasks' },
			{ label: 'Revue sécurité quotidienne', tag: 'NAT-611' },
		])
		expect(breadcrumbs('/tasks/nat-999', NAMES)[1]).toEqual({
			label: 'NAT-999',
			tag: 'NAT-999',
		})
	})

	it('names agents, sources and routines from the lookups', () => {
		expect(breadcrumbs('/agents/dev/skills', NAMES)).toEqual([
			{ label: 'Agents', to: '/agents' },
			{ label: 'Dev (cycle autonome)' },
		])
		expect(breadcrumbs('/sources/dns/pipeline', NAMES)[1]?.label).toBe(
			'Nouveaux sous-domaines',
		)
		expect(breadcrumbs('/routines/analyse', NAMES)[1]?.label).toBe(
			'Analyse board',
		)
	})

	it('names the creation pages', () => {
		expect(breadcrumbs('/agents/nouveau').map((c) => c.label)).toEqual([
			'Agents',
			'Nouvel agent',
		])
		expect(breadcrumbs('/routines/nouvelle')[1]?.label).toBe('Nouvelle routine')
		expect(breadcrumbs('/sources/nouvelle')[1]?.label).toBe('Nouvelle source')
	})

	it('keeps an unknown id as written', () => {
		expect(breadcrumbs('/agents/abc%20123', NAMES)[1]?.label).toBe('abc 123')
	})
})
