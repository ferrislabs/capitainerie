import { describe, expect, it } from 'vitest'
import {
	formatContextWindow,
	formatPrice,
	formatPricePerMtok,
} from '#/domain/settings/format'

describe('formatContextWindow', () => {
	it('compacts token counts', () => {
		expect(formatContextWindow(1_000_000)).toBe('1M')
		expect(formatContextWindow(1_048_576)).toBe('1M')
		expect(formatContextWindow(200_000)).toBe('200k')
		expect(formatContextWindow(131_072)).toBe('128k')
		expect(formatContextWindow(32_768)).toBe('32k')
		expect(formatContextWindow(1_500_000)).toBe('1,5M')
		expect(formatContextWindow(512)).toBe('512')
	})

	it('shows a dash when unknown', () => {
		expect(formatContextWindow(null)).toBe('—')
		expect(formatContextWindow(undefined)).toBe('—')
		expect(formatContextWindow(0)).toBe('—')
	})
})

describe('formatPrice', () => {
	it('converts micros per million tokens to dollars', () => {
		expect(formatPricePerMtok(4_000_000)).toBe('4,00 $')
		expect(formatPricePerMtok(250_000)).toBe('0,25 $')
		expect(
			formatPrice({
				input_per_mtok_micros: 4_000_000,
				output_per_mtok_micros: 20_000_000,
			}),
		).toBe('4,00 $ / 20,00 $')
	})

	it('shows a dash when unknown', () => {
		expect(formatPrice(null)).toBe('—')
		expect(formatPrice(undefined)).toBe('—')
		expect(formatPrice({ input_per_mtok_micros: null })).toBe('—')
		expect(formatPrice({ input_per_mtok_micros: 1_000_000 })).toBe('1,00 $ / —')
	})
})
