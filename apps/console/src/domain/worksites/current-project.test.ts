import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import {
	CURRENT_PROJECT_STORAGE_KEY,
	resolveCurrentProject,
	useCurrentProject,
} from '#/domain/worksites/current-project'

afterEach(() => window.localStorage.clear())

describe('useCurrentProject', () => {
	it('starts empty and remembers the choice for the next visit', () => {
		const first = renderHook(() => useCurrentProject())
		expect(first.result.current[0]).toBe('')

		act(() => first.result.current[1]('p1'))
		expect(first.result.current[0]).toBe('p1')
		expect(window.localStorage.getItem(CURRENT_PROJECT_STORAGE_KEY)).toBe('p1')

		const second = renderHook(() => useCurrentProject())
		expect(second.result.current[0]).toBe('p1')
	})

	it('forgets the choice when emptied', () => {
		const { result } = renderHook(() => useCurrentProject())
		act(() => result.current[1]('p1'))

		act(() => result.current[1](''))

		expect(window.localStorage.getItem(CURRENT_PROJECT_STORAGE_KEY)).toBeNull()
	})
})

describe('resolveCurrentProject', () => {
	const projects = [
		{ id: 'p1', archived: false },
		{ id: 'p2', archived: true },
	]

	it('keeps a project that still exists', () => {
		expect(resolveCurrentProject('p1', projects)).toBe('p1')
	})

	it('drops an archived, deleted or unset project', () => {
		expect(resolveCurrentProject('p2', projects)).toBe('')
		expect(resolveCurrentProject('gone', projects)).toBe('')
		expect(resolveCurrentProject('', projects)).toBe('')
	})
})
