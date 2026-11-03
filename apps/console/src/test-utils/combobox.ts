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

export async function listValues(
	user: UserEvent,
	control: HTMLElement,
): Promise<string[]> {
	await user.click(control)
	const values = optionValues().filter(Boolean)
	await user.keyboard('{Escape}')
	return values
}

export function textOf(control: HTMLElement): string {
	return control instanceof HTMLInputElement
		? control.value
		: (control.textContent ?? '')
}
