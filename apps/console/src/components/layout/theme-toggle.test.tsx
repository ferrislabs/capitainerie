import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ThemeToggle } from '#/components/layout/theme-toggle'

describe('ThemeToggle', () => {
	it('marks the current preference as pressed', () => {
		render(<ThemeToggle preference="system" onChange={vi.fn()} />)

		expect(
			screen
				.getByRole('button', { name: 'Thème du système' })
				.getAttribute('aria-pressed'),
		).toBe('true')
		expect(
			screen
				.getByRole('button', { name: 'Thème sombre' })
				.getAttribute('aria-pressed'),
		).toBe('false')
	})

	it('reports the chosen preference', async () => {
		const onChange = vi.fn()
		render(<ThemeToggle preference="light" onChange={onChange} />)

		await userEvent.click(screen.getByRole('button', { name: 'Thème sombre' }))

		expect(onChange).toHaveBeenCalledWith('dark')
	})
})
