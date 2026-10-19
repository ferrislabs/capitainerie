import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'

import { Combobox, Select } from '#/components/ui/combobox'
import type { ComboboxItem } from '#/components/ui/combobox-model'
import { Field } from '#/components/ui/field'

const ITEMS: ComboboxItem[] = [
	{ value: 'ete', label: 'Été' },
	{ value: 'ecu', label: 'Écureuil' },
	{ value: 'ancre', label: 'Ancre' },
	{ value: 'baleine', label: 'Baleine', disabled: true },
	{ value: 'cap', label: 'Cap' },
]

const GROUPED: ComboboxItem[] = [
	{
		label: 'Anthropic',
		options: [
			{ value: 'opus', label: 'Opus' },
			{ value: 'sonnet', label: 'Sonnet' },
		],
	},
	{ label: 'OpenAI', options: [{ value: 'gpt', label: 'GPT' }] },
]

function Single({ initial = '' }: { initial?: string }) {
	const [value, setValue] = useState(initial)
	return (
		<>
			<Field label="Navire">
				<Combobox
					items={ITEMS}
					value={value}
					onValueChange={setValue}
					noneLabel="Aucun"
					clearable
				/>
			</Field>
			<output data-testid="value">{value}</output>
		</>
	)
}

function Multi({ onChange }: { onChange?: (value: string[]) => void }) {
	const [value, setValue] = useState<string[]>([])
	return (
		<Field label="Étiquettes">
			<Combobox
				multiple
				items={ITEMS}
				value={value}
				onValueChange={(next) => {
					setValue(next)
					onChange?.(next)
				}}
			/>
		</Field>
	)
}

const optionNames = () =>
	screen.queryAllByRole('option').map((option) => option.textContent)

describe('Combobox', () => {
	it('exposes the combobox pattern and ties the label to the input', async () => {
		const user = userEvent.setup()
		render(<Single />)
		const input = screen.getByRole('combobox', { name: 'Navire' })
		expect(input).toHaveProperty('tagName', 'INPUT')
		expect(input.getAttribute('aria-haspopup')).toBe('listbox')
		expect(input.getAttribute('aria-expanded')).toBe('false')
		expect(input.getAttribute('aria-controls')).toBeNull()

		await user.click(input)
		const listbox = screen.getByRole('listbox')
		expect(input.getAttribute('aria-expanded')).toBe('true')
		expect(input.getAttribute('aria-controls')).toBe(listbox.id)
		expect(input.getAttribute('aria-autocomplete')).toBe('list')
		expect(within(listbox).getAllByRole('option')).toHaveLength(6)
	})

	it('filters ignoring case and accents', async () => {
		const user = userEvent.setup()
		render(<Single />)
		const input = screen.getByRole('combobox')
		await user.click(input)
		await user.type(input, 'ECU')
		expect(optionNames()).toEqual(['Écureuil'])
		await user.clear(input)
		await user.type(input, 'ete')
		expect(optionNames()).toEqual(['Été'])
	})

	it('replaces the displayed label when typing over a selected value', async () => {
		const user = userEvent.setup()
		render(<Single initial="ancre" />)
		const input = screen.getByRole('combobox') as HTMLInputElement
		expect(input.value).toBe('Ancre')
		await user.click(input)
		await user.keyboard('cap')
		expect(input.value).toBe('cap')
		expect(optionNames()).toEqual(['Cap'])
	})

	it('shows the empty state when nothing matches', async () => {
		const user = userEvent.setup()
		render(<Single />)
		await user.type(screen.getByRole('combobox'), 'zzz')
		expect(optionNames()).toEqual([])
		expect(screen.getByText('Aucun résultat')).toBeTruthy()
	})

	it('moves with arrows, skips disabled options, wraps and selects with Enter', async () => {
		const user = userEvent.setup()
		render(<Single />)
		const input = screen.getByRole('combobox')
		await user.click(input)
		const active = () =>
			document.getElementById(input.getAttribute('aria-activedescendant') ?? '')
				?.textContent
		expect(active()).toBe('Aucun')
		await user.keyboard('{ArrowDown}')
		expect(active()).toBe('Été')
		await user.keyboard('{ArrowDown}{ArrowDown}')
		expect(active()).toBe('Ancre')
		await user.keyboard('{ArrowDown}')
		expect(active()).toBe('Cap')
		await user.keyboard('{ArrowDown}')
		expect(active()).toBe('Aucun')
		await user.keyboard('{ArrowUp}')
		expect(active()).toBe('Cap')
		await user.keyboard('{Enter}')
		expect(screen.getByTestId('value').textContent).toBe('cap')
		expect(screen.queryByRole('listbox')).toBeNull()
		expect((input as HTMLInputElement).value).toBe('Cap')
	})

	it('jumps to the edges with Home and End when the text is empty', async () => {
		const user = userEvent.setup()
		render(<Single />)
		const input = screen.getByRole('combobox')
		await user.click(input)
		const active = () =>
			document.getElementById(input.getAttribute('aria-activedescendant') ?? '')
				?.textContent
		await user.keyboard('{End}')
		expect(active()).toBe('Cap')
		await user.keyboard('{Home}')
		expect(active()).toBe('Aucun')
	})

	it('closes on Escape without changing the value', async () => {
		const user = userEvent.setup()
		render(<Single initial="ancre" />)
		const input = screen.getByRole('combobox')
		await user.click(input)
		await user.keyboard('{ArrowDown}{Escape}')
		expect(screen.queryByRole('listbox')).toBeNull()
		expect(screen.getByTestId('value').textContent).toBe('ancre')
	})

	it('marks the selected option and offers the none option', async () => {
		const user = userEvent.setup()
		render(<Single initial="ancre" />)
		await user.click(screen.getByRole('combobox'))
		const selected = screen
			.getAllByRole('option')
			.filter((option) => option.getAttribute('aria-selected') === 'true')
		expect(selected.map((option) => option.textContent)).toEqual(['Ancre'])
		await user.click(screen.getByRole('option', { name: 'Aucun' }))
		expect(screen.getByTestId('value').textContent).toBe('')
	})

	it('does not select a disabled option', async () => {
		const user = userEvent.setup()
		render(<Single />)
		await user.click(screen.getByRole('combobox'))
		const disabled = screen.getByRole('option', { name: 'Baleine' })
		expect(disabled.getAttribute('aria-disabled')).toBe('true')
		await user.click(disabled)
		expect(screen.getByTestId('value').textContent).toBe('')
	})

	it('clears the selection with the clear button', async () => {
		const user = userEvent.setup()
		render(<Single initial="cap" />)
		await user.click(
			screen.getByRole('button', { name: 'Effacer la sélection' }),
		)
		expect(screen.getByTestId('value').textContent).toBe('')
	})

	it('renders option groups with labelled group roles', async () => {
		const user = userEvent.setup()
		render(
			<Combobox
				items={GROUPED}
				value=""
				onValueChange={() => {}}
				aria-label="Modèle"
			/>,
		)
		await user.click(screen.getByRole('combobox'))
		const groups = screen.getAllByRole('group')
		expect(
			groups.map((group) => group.getAttribute('aria-labelledby')),
		).not.toContain(null)
		expect(
			within(groups[0] as HTMLElement).getAllByRole('option'),
		).toHaveLength(2)
		expect(screen.getByText('OpenAI')).toBeTruthy()
		await user.keyboard('gpt')
		expect(screen.queryByText('Anthropic')).toBeNull()
		expect(optionNames()).toEqual(['GPT'])
	})

	it('shows a loading state', async () => {
		const user = userEvent.setup()
		render(
			<Combobox
				items={[]}
				value=""
				onValueChange={() => {}}
				loading
				aria-label="Dépôt"
			/>,
		)
		await user.click(screen.getByRole('combobox'))
		expect(screen.getByRole('listbox').getAttribute('aria-busy')).toBe('true')
		expect(screen.getAllByText('Chargement…').length).toBeGreaterThan(0)
	})

	it('wires hint, error and invalid state from the Field', () => {
		render(
			<Field
				label="Navire"
				hint="Le port d’attache"
				error="Choisissez un navire"
			>
				<Combobox items={ITEMS} value="" onValueChange={() => {}} />
			</Field>,
		)
		const input = screen.getByRole('combobox', { name: /Navire/ })
		expect(input.getAttribute('aria-invalid')).toBe('true')
		const described = (input.getAttribute('aria-describedby') ?? '').split(' ')
		const texts = described.map(
			(id) => document.getElementById(id)?.textContent,
		)
		expect(texts.join(' ')).toContain('Choisissez un navire')
		expect(texts.join(' ')).toContain('Le port d’attache')
	})
})

describe('Combobox keys', () => {
	it('leaves Home and End to the caret while the list is closed', async () => {
		const user = userEvent.setup()
		render(<Single />)
		const input = screen.getByRole('combobox')
		input.focus()
		await user.keyboard('{End}')
		expect(screen.queryByRole('listbox')).toBeNull()
	})
})

describe('Combobox multiple', () => {
	it('keeps the list open, shows chips and removes them', async () => {
		const user = userEvent.setup()
		const onChange = vi.fn()
		render(<Multi onChange={onChange} />)
		const input = screen.getByRole('combobox', { name: 'Étiquettes' })
		await user.click(input)
		expect(
			screen.getByRole('listbox').getAttribute('aria-multiselectable'),
		).toBe('true')
		await user.click(screen.getByRole('option', { name: 'Ancre' }))
		await user.click(screen.getByRole('option', { name: 'Cap' }))
		expect(screen.getByRole('listbox')).toBeTruthy()
		expect(onChange).toHaveBeenLastCalledWith(['ancre', 'cap'])
		expect(screen.getByRole('button', { name: 'Retirer Ancre' })).toBeTruthy()
		expect(screen.getByRole('button', { name: 'Retirer Cap' })).toBeTruthy()
		expect(
			screen
				.getByRole('option', { name: 'Ancre' })
				.getAttribute('aria-selected'),
		).toBe('true')

		await user.click(screen.getByRole('button', { name: 'Retirer Ancre' }))
		expect(onChange).toHaveBeenLastCalledWith(['cap'])
		expect(screen.queryByRole('button', { name: 'Retirer Ancre' })).toBeNull()
	})

	it('toggles with the keyboard and removes the last chip with Backspace', async () => {
		const user = userEvent.setup()
		const onChange = vi.fn()
		render(<Multi onChange={onChange} />)
		const input = screen.getByRole('combobox')
		await user.click(input)
		await user.keyboard('{Enter}')
		expect(onChange).toHaveBeenLastCalledWith(['ete'])
		await user.keyboard('{ArrowDown}{Enter}')
		expect(onChange).toHaveBeenLastCalledWith(['ete', 'ecu'])
		await user.keyboard('{Backspace}')
		expect(onChange).toHaveBeenLastCalledWith(['ete'])
		await user.keyboard('{Enter}')
		expect(onChange).toHaveBeenLastCalledWith(['ete', 'ecu'])
	})

	it('moves between chips with arrows and removes the focused one', async () => {
		const user = userEvent.setup()
		const onChange = vi.fn()
		function Chips() {
			const [value, setValue] = useState(['ete', 'ancre', 'cap'])
			return (
				<Combobox
					multiple
					aria-label="Étiquettes"
					items={ITEMS}
					value={value}
					onValueChange={(next) => {
						setValue(next)
						onChange(next)
					}}
				/>
			)
		}
		render(<Chips />)
		const input = screen.getByRole('combobox')
		input.focus()
		await user.keyboard('{ArrowLeft}')
		expect(document.activeElement?.getAttribute('aria-label')).toBe('Cap')
		await user.keyboard('{ArrowLeft}')
		expect(document.activeElement?.getAttribute('aria-label')).toBe('Ancre')
		await user.keyboard('{Backspace}')
		expect(onChange).toHaveBeenLastCalledWith(['ete', 'cap'])
		expect(document.activeElement?.getAttribute('aria-label')).toBe('Été')
		await user.keyboard('{ArrowRight}{Delete}')
		expect(onChange).toHaveBeenLastCalledWith(['ete'])
		expect(document.activeElement).toBe(input)
	})

	it('clears all chips', async () => {
		const user = userEvent.setup()
		const onChange = vi.fn()
		function Cleared() {
			const [value, setValue] = useState(['ancre', 'cap'])
			return (
				<Combobox
					multiple
					clearable
					aria-label="Étiquettes"
					items={ITEMS}
					value={value}
					onValueChange={(next) => {
						setValue(next)
						onChange(next)
					}}
				/>
			)
		}
		render(<Cleared />)
		await user.click(screen.getByRole('button', { name: 'Tout effacer' }))
		expect(onChange).toHaveBeenCalledWith([])
		expect(screen.queryByRole('button', { name: /Retirer/ })).toBeNull()
	})
})

describe('Select', () => {
	function Plain() {
		const [value, setValue] = useState('')
		return (
			<Field label="Priorité">
				<Select
					items={[
						{ value: 'low', label: 'Basse' },
						{ value: 'high', label: 'Haute' },
						{ value: 'urgent', label: 'Urgente' },
					]}
					value={value}
					onValueChange={setValue}
					placeholder="Choisir…"
				/>
			</Field>
		)
	}

	it('is a button combobox labelled by the Field', async () => {
		const user = userEvent.setup()
		render(<Plain />)
		const trigger = screen.getByRole('combobox', { name: 'Priorité' })
		expect(trigger.tagName).toBe('BUTTON')
		expect(trigger.textContent).toBe('Choisir…')
		await user.click(trigger)
		expect(screen.getAllByRole('option')).toHaveLength(3)
		await user.click(screen.getByRole('option', { name: 'Haute' }))
		expect(trigger.textContent).toBe('Haute')
	})

	it('opens with Enter, supports typeahead and picks with Enter', async () => {
		const user = userEvent.setup()
		render(<Plain />)
		const trigger = screen.getByRole('combobox')
		trigger.focus()
		await user.keyboard('{Enter}')
		expect(trigger.getAttribute('aria-expanded')).toBe('true')
		await user.keyboard('u')
		expect(
			document.getElementById(
				trigger.getAttribute('aria-activedescendant') ?? '',
			)?.textContent,
		).toBe('Urgente')
		await user.keyboard('{Enter}')
		expect(trigger.textContent).toBe('Urgente')
		expect(trigger.getAttribute('aria-expanded')).toBe('false')
	})

	it('supports Home, End and Escape', async () => {
		const user = userEvent.setup()
		render(<Plain />)
		const trigger = screen.getByRole('combobox')
		trigger.focus()
		await user.keyboard('{ArrowDown}')
		expect(trigger.getAttribute('aria-expanded')).toBe('true')
		await user.keyboard('{End}')
		expect(
			document.getElementById(
				trigger.getAttribute('aria-activedescendant') ?? '',
			)?.textContent,
		).toBe('Urgente')
		await user.keyboard('{Home}')
		expect(
			document.getElementById(
				trigger.getAttribute('aria-activedescendant') ?? '',
			)?.textContent,
		).toBe('Basse')
		await user.keyboard('{Escape}')
		expect(trigger.getAttribute('aria-expanded')).toBe('false')
		expect(trigger.textContent).toBe('Choisir…')
	})
})
