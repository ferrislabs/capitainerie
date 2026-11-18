import { describe, expect, it } from 'vitest'
import { isValidModelId } from '#/domain/agent-team/model-id'

describe('isValidModelId', () => {
	it('accepts an empty value and real model ids', () => {
		expect(isValidModelId('')).toBe(true)
		expect(isValidModelId('claude-sonnet-5-5')).toBe(true)
		expect(isValidModelId('mistral-large-latest')).toBe(true)
		expect(isValidModelId('models/gemini-3-flash-preview')).toBe(true)
		expect(isValidModelId('llama3.1:8b')).toBe(true)
	})

	it('rejects display names and odd characters', () => {
		expect(isValidModelId('Claude Sonnet 5.5')).toBe(false)
		expect(isValidModelId('-model')).toBe(false)
		expect(isValidModelId('a$b')).toBe(false)
	})
})
