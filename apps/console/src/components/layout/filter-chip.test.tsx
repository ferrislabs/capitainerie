import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { FilterChip } from '#/components/layout/filter-chip'

function setup(selected: ReadonlyArray<string> = []) {
	const onChange = vi.fn()
	render(
		<FilterChip
			label="Source"
			options={['Pipeline', 'Exécutant']}
			selected={selected}
			onChange={onChange}
		/>,
	)
	return onChange
}

describe('FilterChip', () => {
	it('is disabled without options', () => {
		render(
			<FilterChip
				label="Modèle"
				options={[]}
				selected={[]}
				onChange={vi.fn()}
			/>,
		)

		expect(
			(screen.getByRole('button', { name: 'Modèle' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})

	it('has a dashed border while empty', () => {
		setup()

		const chip = screen.getByRole('button', { name: 'Source' })
		expect(chip.className).toContain('border-dashed')
	})

	it('has a solid border and names its values once filled', () => {
		setup(['Pipeline'])

		const chip = screen.getByRole('button', { name: /Source/ })
		expect(chip.className).not.toContain('border-dashed')
		expect(chip.textContent).toContain('Pipeline')
	})

	it('counts several values', () => {
		setup(['Pipeline', 'Exécutant'])

		expect(
			screen.getByRole('button', { name: /Source/ }).textContent,
		).toContain('2')
	})

	it('toggles options with checkboxes', async () => {
		const onChange = setup(['Pipeline'])

		await userEvent.click(screen.getByRole('button', { name: /Source/ }))
		const pipeline = screen.getByRole('menuitemcheckbox', { name: 'Pipeline' })
		expect(pipeline.getAttribute('aria-checked')).toBe('true')
		await userEvent.click(
			screen.getByRole('menuitemcheckbox', { name: 'Exécutant' }),
		)

		expect(onChange).toHaveBeenCalledWith(['Pipeline', 'Exécutant'])
	})

	it('clears the selection', async () => {
		const onChange = setup(['Pipeline'])

		await userEvent.click(screen.getByRole('button', { name: /Source/ }))
		await userEvent.click(screen.getByRole('menuitem', { name: 'Effacer' }))

		expect(onChange).toHaveBeenCalledWith([])
	})
})
