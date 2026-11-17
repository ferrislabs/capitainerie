import { describe, expect, it } from 'vitest'
import { breadcrumbs } from '#/components/layout/breadcrumbs'

describe('breadcrumbs', () => {
	it('shows only the home label on the home page', () => {
		expect(breadcrumbs('/')).toEqual([{ label: 'Accueil' }])
	})

	it('links every crumb but the last', () => {
		expect(breadcrumbs('/settings/providers')).toEqual([
			{ label: 'Accueil', to: '/' },
			{ label: 'Paramètres', to: '/settings' },
			{ label: 'Fournisseurs' },
		])
	})

	it('names the team pages', () => {
		expect(breadcrumbs('/agents/equipe').map((c) => c.label)).toEqual([
			'Accueil',
			'Agents',
			'Équipe',
		])
	})

	it('names the tab after an id, not as a section', () => {
		expect(breadcrumbs('/sources/dns/pipeline').map((c) => c.label)).toEqual([
			'Accueil',
			'Sources',
			'dns',
			'Pipeline',
		])
	})

	it('names the first-level tab of a tabbed section', () => {
		expect(breadcrumbs('/audit/runs').map((c) => c.label)).toEqual([
			'Accueil',
			'Audit',
			'Exécutions',
		])
	})

	it('keeps an unknown segment as written', () => {
		expect(breadcrumbs('/runs/abc%20123')).toEqual([
			{ label: 'Accueil', to: '/' },
			{ label: 'Runs', to: '/runs' },
			{ label: 'abc 123' },
		])
	})
})
