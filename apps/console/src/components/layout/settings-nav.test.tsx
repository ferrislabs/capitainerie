import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { SettingsNav } from '#/components/layout/settings-nav'
import { renderAtPath } from '#/components/layout/test-router'
import { SETTINGS_NAV } from '#/domain/settings/nav'

describe('SettingsNav', () => {
	it('offers Fournisseurs and Modèles and marks the current one', async () => {
		await renderAtPath(
			<SettingsNav
				title="Paramètres"
				groupLabel="Organisation"
				items={SETTINGS_NAV}
			/>,
			'/settings/models',
		)

		expect(screen.getByRole('navigation', { name: 'Paramètres' })).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Fournisseurs' })
				.getAttribute('aria-current'),
		).toBeNull()
		expect(
			screen
				.getByRole('link', { name: 'Modèles' })
				.getAttribute('aria-current'),
		).toBe('page')
	})
})
