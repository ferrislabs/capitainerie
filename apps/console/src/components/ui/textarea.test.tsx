import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Textarea } from '#/components/ui/textarea'

function mockScrollHeight(height: number) {
	vi.spyOn(
		HTMLTextAreaElement.prototype,
		'scrollHeight',
		'get',
	).mockImplementation(() => height)
}

describe('Textarea', () => {
	afterEach(() => {
		vi.restoreAllMocks()
	})

	it('grows to fit its content', () => {
		mockScrollHeight(90)
		render(<Textarea autoGrow defaultValue="a\nb\nc" aria-label="Note" />)
		const area = screen.getByLabelText('Note')
		expect(area.style.height).toBe('90px')
		expect(area.style.overflowY).toBe('hidden')
	})

	it('stops growing at maxRows and scrolls', () => {
		mockScrollHeight(400)
		render(<Textarea maxRows={4} defaultValue="long" aria-label="Note" />)
		const area = screen.getByLabelText('Note')
		expect(area.style.height).toBe('80px')
		expect(area.style.overflowY).toBe('auto')
	})

	it('recomputes the height on input', async () => {
		const user = userEvent.setup()
		mockScrollHeight(40)
		render(<Textarea autoGrow aria-label="Note" />)
		const area = screen.getByLabelText('Note')
		expect(area.style.height).toBe('40px')
		mockScrollHeight(120)
		await user.type(area, 'x')
		expect(area.style.height).toBe('120px')
	})

	it('does not manage height by default', () => {
		mockScrollHeight(300)
		render(<Textarea aria-label="Note" />)
		expect(screen.getByLabelText('Note').style.height).toBe('')
	})

	it('counts characters against maxLength', async () => {
		const user = userEvent.setup()
		render(<Textarea showCount maxLength={10} aria-label="Note" />)
		expect(screen.getByText('0 / 10')).toBeTruthy()
		await user.type(screen.getByLabelText('Note'), 'abc')
		expect(screen.getByText('3 / 10')).toBeTruthy()
	})
})
