import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import {
	EXECUTOR_MODEL_MESSAGE,
	useSaveAssignments,
} from '#/domain/settings/hooks/use-save-assignments'
import { ADMIN_ONLY_MODELS } from '#/domain/settings/providers'
import { clearAuth, setAuth } from '#/store/auth.store'

const QWEN = {
	provider: 'openai_compatible',
	model: 'qwen2.5:7b-instruct',
} as const
const ASSIGNMENTS = {
	extractor: QWEN,
	judge: QWEN,
	writer: QWEN,
	embeddings: { provider: 'openai_compatible', model: 'bge-m3:latest' },
} as const

describe('useSaveAssignments', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		harness.seed('/api/v1/model-assignments', { assignments: null })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('puts the four roles and updates the cached assignments', async () => {
		fetchMock.mockResolvedValue(json(200, { assignments: ASSIGNMENTS }))
		const onSuccess = vi.fn()
		const { result } = renderHook(() => useSaveAssignments(), {
			wrapper: harness.wrapper,
		})

		act(() => result.current.save(ASSIGNMENTS, { onSuccess }))

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/model-assignments')
		expect(init.method).toBe('PUT')
		expect(JSON.parse(String(init.body))).toEqual(ASSIGNMENTS)
		expect(onSuccess).toHaveBeenCalledOnce()
		expect(
			harness.client.getQueryData([{ _id: '/api/v1/model-assignments' }]),
		).toEqual({ assignments: ASSIGNMENTS })
		expect(harness.isInvalidated('/api/v1/model-assignments')).toBe(true)
	})

	it('surfaces the validation message on 422', async () => {
		fetchMock.mockResolvedValue(
			json(422, {
				code: 'unprocessable',
				status: 422,
				message: 'provider mistral is not configured',
			}),
		)
		const { result } = renderHook(() => useSaveAssignments(), {
			wrapper: harness.wrapper,
		})

		act(() => result.current.save(ASSIGNMENTS))

		await waitFor(() =>
			expect(result.current.errorMessage).toBe(
				'provider mistral is not configured',
			),
		)
		expect(
			harness.client.getQueryData([{ _id: '/api/v1/model-assignments' }]),
		).toEqual({ assignments: null })
	})

	it('explains a 403 as admin-only', async () => {
		fetchMock.mockResolvedValue(
			json(403, { code: 'forbidden', status: 403, message: 'forbidden' }),
		)
		const { result } = renderHook(() => useSaveAssignments(), {
			wrapper: harness.wrapper,
		})

		act(() => result.current.save(ASSIGNMENTS))

		await waitFor(() =>
			expect(result.current.errorMessage).toBe(ADMIN_ONLY_MODELS),
		)
	})

	it('sends the executor and explains a refused one', async () => {
		fetchMock.mockResolvedValue(
			json(422, {
				code: 'invalid',
				status: 422,
				message: 'the executor model must be a claude-* model',
			}),
		)
		const { result } = renderHook(() => useSaveAssignments(), {
			wrapper: harness.wrapper,
		})

		act(() => result.current.save({ ...ASSIGNMENTS, executor: 'gpt-4o' }))

		await waitFor(() =>
			expect(result.current.errorMessage).toBe(EXECUTOR_MODEL_MESSAGE),
		)
		const [, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(JSON.parse(String(init.body))).toEqual({
			...ASSIGNMENTS,
			executor: 'gpt-4o',
		})
	})
})
