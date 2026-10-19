import { describe, expect, it } from 'vitest'
import { agentName, stepKindLabel } from '#/domain/runs/step-labels'

describe('stepKindLabel', () => {
	it('names the four pipeline steps in French', () => {
		expect(stepKindLabel('extract')).toBe('Extraction')
		expect(stepKindLabel('deduplicate')).toBe('Dédoublonnage')
		expect(stepKindLabel('write')).toBe('Rédaction')
		expect(stepKindLabel('execute')).toBe('Exécution')
	})

	it('shows an unknown kind as it is', () => {
		expect(stepKindLabel('other')).toBe('other')
	})
})

describe('agentName', () => {
	it('drops the client id prefix', () => {
		expect(agentName('capitainerie-agent-writer')).toBe('writer')
		expect(agentName('extractor')).toBe('extractor')
	})
})
