import { describe, expect, it } from 'vitest'
import { breadcrumbs, type EntityNames } from '#/components/layout/breadcrumbs'

const TASK = '6f1c2a90-1111-4000-8000-000000000001'

const NAMES: EntityNames = {
	task: (id) => (id === TASK ? 'Revue sécurité quotidienne' : undefined),
	agent: (id) => (id === 'dev' ? 'Dev (cycle autonome)' : undefined),
	source: (id) => (id === 's-1' ? 'Pièces Chatwoot' : undefined),
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

	it('names a task with its short id tag', () => {
		expect(breadcrumbs(`/tasks/${TASK.toUpperCase()}`, NAMES)).toEqual([
			{ label: 'Tâches', to: '/tasks' },
			{ label: 'Revue sécurité quotidienne', tag: '6f1c2a90' },
		])
		expect(
			breadcrumbs('/tasks/99999999-0000-4000-8000-000000000009', NAMES)[1],
		).toEqual({ label: '99999999', tag: '99999999' })
	})

	it('names agents and sources from the lookup, routines by id', () => {
		expect(breadcrumbs('/agents/dev/skills', NAMES)).toEqual([
			{ label: 'Agents', to: '/agents' },
			{ label: 'Dev (cycle autonome)' },
		])
		expect(breadcrumbs('/sources/s-1/evenements', NAMES)).toEqual([
			{ label: 'Sources', to: '/sources' },
			{ label: 'Pièces Chatwoot' },
		])
		expect(breadcrumbs('/sources/s-2', NAMES)[1]?.label).toBe('s-2')
		expect(breadcrumbs('/routines/analyse', NAMES)[1]?.label).toBe('analyse')
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
