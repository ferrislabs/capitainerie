import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { UiShowcase } from '#/dev/ui-showcase'

describe('UiShowcase', () => {
	it('lists every component family', () => {
		render(<UiShowcase />)

		for (const title of [
			'TextInput',
			'Textarea',
			'Select et Combobox',
			'DropdownMenu',
			'Checkbox, Switch, RadioGroup',
		]) {
			expect(screen.getByRole('heading', { name: title })).toBeTruthy()
		}
	})

	it('opens the menu with sections, a sub-menu and a destructive item', async () => {
		const user = userEvent.setup()
		render(<UiShowcase />)

		await user.click(screen.getByRole('button', { name: 'Actions' }))

		expect(screen.getByRole('group', { name: 'Carte' })).toBeTruthy()
		const destructive = screen.getByRole('menuitem', { name: /Supprimer/ })
		expect(destructive.getAttribute('data-variant')).toBe('destructive')
		expect(screen.getByRole('menuitem', { name: /Assigner à/ })).toBeTruthy()
	})
})
