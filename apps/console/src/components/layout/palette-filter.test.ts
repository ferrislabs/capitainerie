import { describe, expect, it } from 'vitest'
import {
	filterCommands,
	type PaletteCommand,
} from '#/components/layout/palette-filter'

const COMMANDS: ReadonlyArray<PaletteCommand> = [
	{ id: 'home', group: 'Aller à', label: 'Accueil', to: '/' },
	{ id: 'models', group: 'Aller à', label: 'Modèles', to: '/settings/models' },
	{ id: 'theme', group: 'Action', label: 'Changer de thème' },
]

describe('filterCommands', () => {
	it('returns everything for an empty query', () => {
		expect(filterCommands(COMMANDS, '  ')).toHaveLength(3)
	})

	it('ignores accents and case', () => {
		expect(filterCommands(COMMANDS, 'MODELES').map((c) => c.id)).toEqual([
			'models',
		])
	})

	it('matches the group too', () => {
		expect(filterCommands(COMMANDS, 'action').map((c) => c.id)).toEqual([
			'theme',
		])
	})

	it('returns nothing when no command matches', () => {
		expect(filterCommands(COMMANDS, 'zzz')).toEqual([])
	})
})
