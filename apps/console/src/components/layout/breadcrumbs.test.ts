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

	it('keeps an unknown segment as written', () => {
		expect(breadcrumbs('/runs/abc%20123')).toEqual([
			{ label: 'Accueil', to: '/' },
			{ label: 'Runs', to: '/runs' },
			{ label: 'abc 123' },
		])
	})
})
