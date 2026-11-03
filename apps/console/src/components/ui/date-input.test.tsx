import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'

import { DateInput } from '#/components/ui/date-input'

describe('DateInput', () => {
	it('is a native date input with min and max', () => {
		render(
			<DateInput aria-label="Échéance" min="2026-01-01" max="2026-12-31" />,
		)
		const input = screen.getByLabelText('Échéance') as HTMLInputElement
		expect(input.type).toBe('date')
		expect(input.min).toBe('2026-01-01')
		expect(input.max).toBe('2026-12-31')
	})

	it('offers no clear button while empty', () => {
		render(<DateInput aria-label="Échéance" />)
		expect(screen.queryByRole('button', { name: 'Effacer la date' })).toBeNull()
	})

	it('clears a controlled value', async () => {
		const user = userEvent.setup()
		const onChange = vi.fn()
		function Controlled() {
			const [value, setValue] = useState('2026-03-04')
			return (
				<DateInput
					aria-label="Échéance"
					value={value}
					onChange={(event) => {
						setValue(event.target.value)
						onChange(event.target.value)
					}}
				/>
			)
		}
		render(<Controlled />)
		await user.click(screen.getByRole('button', { name: 'Effacer la date' }))
		expect(onChange).toHaveBeenCalledWith('')
		expect((screen.getByLabelText('Échéance') as HTMLInputElement).value).toBe(
			'',
		)
		expect(screen.queryByRole('button', { name: 'Effacer la date' })).toBeNull()
		expect(document.activeElement).toBe(screen.getByLabelText('Échéance'))
	})

	it('clears an uncontrolled value', async () => {
		const user = userEvent.setup()
		render(<DateInput aria-label="Échéance" defaultValue="2026-03-04" />)
		await user.click(screen.getByRole('button', { name: 'Effacer la date' }))
		expect((screen.getByLabelText('Échéance') as HTMLInputElement).value).toBe(
			'',
		)
	})

	it('does not clear when disabled', () => {
		render(
			<DateInput aria-label="Échéance" defaultValue="2026-03-04" disabled />,
		)
		expect(screen.queryByRole('button', { name: 'Effacer la date' })).toBeNull()
	})
})
