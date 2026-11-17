import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { CommandPalette } from '#/components/layout/command-palette'
import type { PaletteCommand } from '#/components/layout/palette-filter'

const COMMANDS: ReadonlyArray<PaletteCommand> = [
	{ id: 'home', group: 'Aller à', label: 'Accueil', to: '/' },
	{ id: 'runs', group: 'Aller à', label: 'Runs', to: '/runs' },
	{ id: 'theme', group: 'Action', label: 'Changer de thème' },
]

function setup() {
	const onRun = vi.fn()
	const onOpenChange = vi.fn()
	render(
		<CommandPalette
			open
			onOpenChange={onOpenChange}
			commands={COMMANDS}
			onRun={onRun}
		/>,
	)
	return { onRun, onOpenChange }
}

describe('CommandPalette', () => {
	it('lists every command when the query is empty', () => {
		setup()
		expect(screen.getAllByRole('option')).toHaveLength(3)
	})

	it('filters while typing', async () => {
		setup()
		await userEvent.type(screen.getByRole('combobox'), 'run')
		expect(screen.getAllByRole('option').map((o) => o.textContent)).toEqual([
			'RunsAller à',
		])
	})

	it('moves with the arrows and runs the active command on Enter', async () => {
		const { onRun } = setup()
		await userEvent.type(screen.getByRole('combobox'), '{ArrowDown}{Enter}')
		expect(onRun).toHaveBeenCalledWith(COMMANDS[1])
	})

	it('wraps around from the first to the last command', async () => {
		const { onRun } = setup()
		await userEvent.type(screen.getByRole('combobox'), '{ArrowUp}{Enter}')
		expect(onRun).toHaveBeenCalledWith(COMMANDS[2])
	})

	it('says so when nothing matches', async () => {
		setup()
		await userEvent.type(screen.getByRole('combobox'), 'zzz')
		expect(screen.getByText('Aucun résultat')).toBeTruthy()
	})
})
