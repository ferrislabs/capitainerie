import { act, renderHook } from '@testing-library/react'
import type { DragEvent } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { useFileDrop } from '#/domain/runs/hooks/use-file-drop'

function dragEvent(files: File[] = []) {
	return {
		preventDefault: vi.fn(),
		relatedTarget: null,
		currentTarget: document.createElement('div'),
		dataTransfer: { files },
	} as unknown as DragEvent<HTMLElement>
}

describe('useFileDrop', () => {
	it('flags the drag and forwards the first dropped file', () => {
		const onFile = vi.fn()
		const { result } = renderHook(() => useFileDrop(onFile, false))
		const file = new File(['x'], 'a.txt')

		act(() => result.current.handlers.onDragEnter(dragEvent()))
		expect(result.current.isDragging).toBe(true)

		const drop = dragEvent([file])
		act(() => result.current.handlers.onDrop(drop))
		expect(drop.preventDefault).toHaveBeenCalled()
		expect(result.current.isDragging).toBe(false)
		expect(onFile).toHaveBeenCalledWith(file)
	})

	it('clears the flag when the pointer leaves', () => {
		const { result } = renderHook(() => useFileDrop(vi.fn(), false))

		act(() => result.current.handlers.onDragEnter(dragEvent()))
		act(() => result.current.handlers.onDragLeave(dragEvent()))

		expect(result.current.isDragging).toBe(false)
	})

	it('ignores everything while disabled', () => {
		const onFile = vi.fn()
		const { result } = renderHook(() => useFileDrop(onFile, true))

		act(() => result.current.handlers.onDragEnter(dragEvent()))
		act(() =>
			result.current.handlers.onDrop(dragEvent([new File(['x'], 'a.txt')])),
		)

		expect(result.current.isDragging).toBe(false)
		expect(onFile).not.toHaveBeenCalled()
	})
})
