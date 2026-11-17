import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { PRESETS } from '#/domain/sources/criteria'
import {
	pipelineActions,
	resetPipelines,
	unusedCriteria,
	usePipeline,
} from '#/domain/sources/mock/pipeline-store'

describe('pipeline store', () => {
	beforeEach(resetPipelines)

	it('starts on the Prospection B2B preset with a dry run', () => {
		const { result } = renderHook(() => usePipeline('s'))
		expect(result.current.criteria).toHaveLength(6)
		expect(result.current.dry).toBe(true)
		expect(result.current.destination).toBe('Odoo (MCP)')
	})

	it('keeps one config per source', () => {
		const a = renderHook(() => usePipeline('a'))
		const b = renderHook(() => usePipeline('b'))
		act(() => pipelineActions.remove('a', 0))
		expect(a.result.current.criteria).toHaveLength(5)
		expect(b.result.current.criteria).toHaveLength(6)
	})

	it('toggles, edits, adds and applies presets', () => {
		const { result } = renderHook(() => usePipeline('s'))
		act(() => pipelineActions.toggle('s', 0))
		expect(result.current.criteria[0]?.enabled).toBe(false)
		act(() => pipelineActions.setOnFail('s', 0, 'ignore'))
		expect(result.current.criteria[0]?.onFail).toBe('ignore')
		act(() => pipelineActions.setParam('s', 3, '80'))
		expect(result.current.criteria[3]?.param).toBe('80')
		act(() => pipelineActions.applyPreset('s', 'Minimal'))
		expect(result.current.criteria).toHaveLength(2)
		act(() => pipelineActions.add('s', 'age'))
		expect(result.current.criteria[2]).toMatchObject({
			id: 'age',
			param: 30,
			onFail: 'review',
		})
	})

	it('resets', () => {
		const { result } = renderHook(() => usePipeline('s'))
		act(() => pipelineActions.applyPreset('s', 'Minimal'))
		act(resetPipelines)
		expect(result.current.criteria).toHaveLength(6)
	})

	it('lists unused criteria', () => {
		expect(unusedCriteria(PRESETS.Minimal ?? [])).toHaveLength(7)
	})
})
