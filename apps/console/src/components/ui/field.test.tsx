import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { Checkbox } from '#/components/ui/checkbox'
import { Field } from '#/components/ui/field'
import { RadioGroup, RadioGroupItem } from '#/components/ui/radio-group'
import { Switch } from '#/components/ui/switch'
import { Textarea } from '#/components/ui/textarea'
import { TextInput } from '#/components/ui/text-input'

describe('Field', () => {
	it('ties the label to the control', () => {
		render(
			<Field label="Nom">
				<TextInput />
			</Field>,
		)
		expect(screen.getByLabelText('Nom')).toBe(screen.getByRole('textbox'))
	})

	it('keeps a caller id on the control', () => {
		render(
			<Field label="Nom" id="client-name">
				<TextInput />
			</Field>,
		)
		expect(screen.getByRole('textbox').id).toBe('client-name')
	})

	it('links hint and error through aria-describedby and flags invalid', () => {
		render(
			<Field label="Nom" hint="Tel qu’au contrat" error="Le nom est requis">
				<TextInput />
			</Field>,
		)
		const input = screen.getByRole('textbox')
		expect(input.getAttribute('aria-invalid')).toBe('true')
		const ids = (input.getAttribute('aria-describedby') ?? '').split(' ')
		expect(ids).toHaveLength(2)
		const texts = ids.map((id) => document.getElementById(id)?.textContent)
		expect(texts.some((text) => text?.includes('Le nom est requis'))).toBe(true)
		expect(texts).toContain('Tel qu’au contrat')
	})

	it('is valid without error and has no describedby', () => {
		render(
			<Field label="Nom">
				<TextInput />
			</Field>,
		)
		const input = screen.getByRole('textbox')
		expect(input.getAttribute('aria-invalid')).toBeNull()
		expect(input.getAttribute('aria-describedby')).toBeNull()
	})

	it('marks required and optional', () => {
		const { rerender } = render(
			<Field label="Nom" required>
				<TextInput />
			</Field>,
		)
		expect(screen.getByRole('textbox').getAttribute('aria-required')).toBe(
			'true',
		)
		expect(screen.getByText('(obligatoire)')).toBeTruthy()
		rerender(
			<Field label="Nom" optional>
				<TextInput />
			</Field>,
		)
		expect(screen.getByText('(facultatif)')).toBeTruthy()
	})

	it('shows the character counter and warns when over the limit', () => {
		const { rerender } = render(
			<Field label="Note" count={12} max={20}>
				<Textarea />
			</Field>,
		)
		const textbox = screen.getByRole('textbox')
		const counterId = (textbox.getAttribute('aria-describedby') ?? '').split(
			' ',
		)[0]
		const counter = document.getElementById(counterId ?? '')
		expect(counter?.textContent).toBe('12 / 20')
		expect(counter?.getAttribute('data-over')).toBe('false')
		rerender(
			<Field label="Note" count={25} max={20}>
				<Textarea />
			</Field>,
		)
		expect(document.getElementById(counterId ?? '')?.textContent).toContain(
			'trop long',
		)
	})

	it('disables its control', () => {
		render(
			<Field label="Nom" disabled>
				<TextInput />
			</Field>,
		)
		expect((screen.getByRole('textbox') as HTMLInputElement).disabled).toBe(
			true,
		)
	})

	it('labels a group of radios', () => {
		render(
			<Field label="Priorité" group hint="Une seule valeur">
				<RadioGroup>
					<RadioGroupItem value="a" label="Haute" />
					<RadioGroupItem value="b" label="Basse" />
				</RadioGroup>
			</Field>,
		)
		expect(screen.getByRole('radiogroup', { name: 'Priorité' })).toBeTruthy()
		expect(screen.getByRole('radio', { name: 'Haute' })).toBeTruthy()
	})

	it('labels checkbox and switch inline', () => {
		render(
			<>
				<Checkbox label="Critère rempli" description="Coché par l’équipe" />
				<Switch label="Notifications" />
			</>,
		)
		const box = screen.getByRole('checkbox', { name: 'Critère rempli' })
		expect(box.getAttribute('aria-describedby')).toBeTruthy()
		expect(screen.getByRole('switch', { name: 'Notifications' })).toBeTruthy()
	})
})
