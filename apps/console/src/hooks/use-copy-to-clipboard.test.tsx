import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useCopyToClipboard } from '#/hooks/use-copy-to-clipboard'

describe('useCopyToClipboard', () => {
	const writeText = vi.fn()

	beforeEach(() => {
		vi.useFakeTimers()
		writeText.mockReset()
		Object.defineProperty(navigator, 'clipboard', {
			value: { writeText },
			configurable: true,
		})
	})

	afterEach(() => {
		vi.useRealTimers()
	})

	it('copies the text and reports it for two seconds', async () => {
		writeText.mockResolvedValue(undefined)
		const { result } = renderHook(() => useCopyToClipboard())

		await act(async () => result.current.copy('{"a":1}'))

		expect(writeText).toHaveBeenCalledWith('{"a":1}')
		expect(result.current.copied).toBe(true)

		act(() => vi.advanceTimersByTime(2000))
		expect(result.current.copied).toBe(false)
	})

	it('does not report a copy the browser refused', async () => {
		writeText.mockRejectedValue(new Error('denied'))
		const { result } = renderHook(() => useCopyToClipboard())

		await act(async () => result.current.copy('x'))

		expect(result.current.copied).toBe(false)
	})

	it('forgets the copy on reset', async () => {
		writeText.mockResolvedValue(undefined)
		const { result } = renderHook(() => useCopyToClipboard())
		await act(async () => result.current.copy('x'))

		act(() => result.current.reset())

		expect(result.current.copied).toBe(false)
	})
})
