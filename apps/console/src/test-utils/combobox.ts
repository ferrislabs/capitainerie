import { screen, within } from '@testing-library/react'
import type { UserEvent } from '@testing-library/user-event'

export async function chooseValue(
	user: UserEvent,
	control: HTMLElement,
	value: string,
) {
	if (control.getAttribute('aria-expanded') !== 'true') {
		await user.click(control)
	}
	const listbox = await screen.findByRole('listbox')
	const option = within(listbox)
		.getAllByRole('option')
		.find((candidate) => candidate.getAttribute('data-value') === value)
	if (!option) throw new Error(`Aucune option de valeur « ${value} »`)
	await user.click(option)
}

export function optionValues(): string[] {
	return screen
		.queryAllByRole('option')
		.map((option) => option.getAttribute('data-value') ?? '')
}
