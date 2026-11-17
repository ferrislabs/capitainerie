import { act, render, renderHook, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Notice, useNotice } from '#/components/layout/notice'

describe('Notice', () => {
	it('is silent without a message', () => {
		render(<Notice message={null} />)
		expect(screen.getByRole('status').className).toContain('sr-only')
	})

	it('shows the message', () => {
		render(<Notice message="Routine lancée" />)
		expect(screen.getByRole('status').textContent).toBe('Routine lancée')
	})
})

describe('useNotice', () => {
	beforeEach(() => vi.useFakeTimers())
	afterEach(() => vi.useRealTimers())

	it('clears the message after the delay', () => {
		const { result } = renderHook(() => useNotice(1000))
		act(() => result.current[1]('Fait'))
		expect(result.current[0]).toBe('Fait')
		act(() => vi.advanceTimersByTime(1000))
		expect(result.current[0]).toBeNull()
	})

	it('restarts the delay on a new message', () => {
		const { result } = renderHook(() => useNotice(1000))
		act(() => result.current[1]('Un'))
		act(() => vi.advanceTimersByTime(600))
		act(() => result.current[1]('Deux'))
		act(() => vi.advanceTimersByTime(600))
		expect(result.current[0]).toBe('Deux')
	})
})
