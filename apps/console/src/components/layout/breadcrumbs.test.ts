import { describe, expect, it } from 'vitest'
import { breadcrumbs, type EntityNames } from '#/components/layout/breadcrumbs'

const TASK = '6f1c2a90-1111-4000-8000-000000000001'

const NAMES: EntityNames = {
	task: (id) => (id === TASK ? 'Revue sécurité quotidienne' : undefined),
	agent: (id) => (id === 'dev' ? 'Dev (cycle autonome)' : undefined),
	source: (id) => (id === 's-1' ? 'Pièces Chatwoot' : undefined),
	routine: (id) => (id === 'r-1' ? 'Revue hebdomadaire' : undefined),
	artifact: (id) => (id === 'art-1' ? 'Audit sécurité' : undefined),
}

describe('breadcrumbs', () => {
	it('names the dashboard on the home page', () => {
		expect(breadcrumbs('/')).toEqual([{ label: 'Tableau de bord' }])
	})

	it('shows a section page alone, tabs included', () => {
		expect(breadcrumbs('/tasks')).toEqual([{ label: 'Tâches' }])
		expect(breadcrumbs('/audit/runs')).toEqual([{ label: 'Audit' }])
		expect(breadcrumbs('/worksites')).toEqual([{ label: 'Chantiers' }])
		expect(breadcrumbs('/agents/organigramme')).toEqual([{ label: 'Agents' }])
	})

	it('names a task with its short id tag', () => {
		expect(breadcrumbs(`/tasks/${TASK.toUpperCase()}`, NAMES)).toEqual([
			{ label: 'Tâches', to: '/tasks' },
			{ label: 'Revue sécurité quotidienne', tag: '6f1c2a90' },
		])
		expect(
			breadcrumbs('/tasks/99999999-0000-4000-8000-000000000009', NAMES)[1],
		).toEqual({ label: '99999999', tag: '99999999' })
	})

	it('names agents, sources and routines from the lookup', () => {
		expect(breadcrumbs('/agents/dev/skills', NAMES)).toEqual([
			{ label: 'Agents', to: '/agents' },
			{ label: 'Dev (cycle autonome)' },
		])
		expect(breadcrumbs('/sources/s-1/evenements', NAMES)).toEqual([
			{ label: 'Sources', to: '/sources' },
			{ label: 'Pièces Chatwoot' },
		])
		expect(breadcrumbs('/sources/s-2', NAMES)[1]?.label).toBe('s-2')
		expect(breadcrumbs('/routines/r-1/executions', NAMES)).toEqual([
			{ label: 'Routines', to: '/routines' },
			{ label: 'Revue hebdomadaire' },
		])
		expect(breadcrumbs('/routines/r-2', NAMES)[1]?.label).toBe('r-2')
	})

	it('names an artifact by its title once loaded', () => {
		expect(breadcrumbs('/artifacts/art-1', NAMES)).toEqual([
			{ label: 'Artifacts', to: '/artifacts' },
			{ label: 'Audit sécurité' },
		])
		expect(
			breadcrumbs('/artifacts/0a1b2c3d-0000-4000-8000-000000000000')[1]?.label,
		).toBe('0a1b2c3d')
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
