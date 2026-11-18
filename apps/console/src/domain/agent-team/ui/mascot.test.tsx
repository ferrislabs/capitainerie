import { render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { hueOf } from '#/domain/agent-team/agent-identity'
import { Mascot, mascotPalette } from '#/domain/agent-team/ui/mascot'

function mockReducedMotion(reduce: boolean) {
	vi.spyOn(window, 'matchMedia').mockImplementation(
		(query: string) =>
			({
				matches: reduce && query.includes('prefers-reduced-motion'),
				media: query,
				onchange: null,
				addEventListener: () => {},
				removeEventListener: () => {},
				addListener: () => {},
				removeListener: () => {},
				dispatchEvent: () => false,
			}) as MediaQueryList,
	)
}

function mockFrames() {
	const raf = vi
		.spyOn(window, 'requestAnimationFrame')
		.mockImplementation(() => 1)
	const caf = vi
		.spyOn(window, 'cancelAnimationFrame')
		.mockImplementation(() => {})
	return { raf, caf }
}

afterEach(() => {
	vi.restoreAllMocks()
})

describe('Mascot', () => {
	it.each([
		'idle',
		'running',
		'sleep',
	] as const)('renders the %s state', (state) => {
		mockFrames()
		const { container } = render(<Mascot name="Darnold" state={state} />)
		const svg = container.querySelector('svg')
		expect(svg).not.toBeNull()
		expect(svg?.getAttribute('data-state')).toBe(state)
		expect(svg?.getAttribute('aria-hidden')).toBe('true')
	})

	it('exposes an accessible label only when asked', () => {
		mockFrames()
		const { getByRole } = render(<Mascot name="Darnold" label="Mascotte" />)
		expect(getByRole('img', { name: 'Mascotte' })).toBeTruthy()
	})

	it('gives every instance its own gradient ids', () => {
		mockFrames()
		const { container } = render(
			<>
				<Mascot name="Darnold" />
				<Mascot name="Zoé" />
			</>,
		)
		const ids = [...container.querySelectorAll('[id]')].map((el) => el.id)
		expect(ids.length).toBe(6)
		expect(new Set(ids).size).toBe(ids.length)
		for (const svg of container.querySelectorAll('svg')) {
			const own = [...svg.querySelectorAll('[id]')].map((el) => el.id)
			for (const ref of svg.innerHTML.matchAll(/url\(#([^)]+)\)/g))
				expect(own).toContain(ref[1])
		}
	})

	it('derives the colours from the name through hueOf', () => {
		mockFrames()
		const h = hueOf('Darnold')
		const { container } = render(<Mascot name="Darnold" />)
		const stops = [
			...container.querySelectorAll('stop[stop-color^="hsl"]'),
		].map((el) => el.getAttribute('stop-color'))
		expect(stops).toEqual([
			`hsl(${(h + 200) % 360} 78% 76%)`,
			`hsl(${h} 74% 66%)`,
			`hsl(${h} 70% 70%)`,
			`hsl(${h} 66% 60%)`,
		])
		expect(mascotPalette('Darnold')).not.toEqual(mascotPalette('Zoé'))
	})

	it('falls back to a neutral palette without a name', () => {
		expect(mascotPalette('  ').shellBottom).toBe('hsl(0 6% 66%)')
	})

	it('animates through requestAnimationFrame by default', () => {
		mockReducedMotion(false)
		const { raf } = mockFrames()
		render(<Mascot name="Darnold" />)
		expect(raf).toHaveBeenCalled()
	})

	it('does not start the animation loop under reduced motion', () => {
		mockReducedMotion(true)
		const { raf } = mockFrames()
		const { container } = render(<Mascot name="Darnold" />)
		expect(raf).not.toHaveBeenCalled()
		window.dispatchEvent(
			new MouseEvent('pointermove', { clientX: 400, clientY: 0 }),
		)
		expect(raf).not.toHaveBeenCalled()
		const pupil = container.querySelector('[data-part="pupL"]')
		expect(Number(pupil?.getAttribute('cx'))).toBeGreaterThan(82)
	})

	it('removes its listeners and stops the loop on unmount', () => {
		mockReducedMotion(false)
		const { caf } = mockFrames()
		const add = vi.spyOn(window, 'addEventListener')
		const remove = vi.spyOn(window, 'removeEventListener')
		const docAdd = vi.spyOn(document, 'addEventListener')
		const docRemove = vi.spyOn(document, 'removeEventListener')
		const { unmount } = render(<Mascot name="Darnold" />)
		const move = add.mock.calls.find(([type]) => type === 'pointermove')
		const visibility = docAdd.mock.calls.find(
			([type]) => type === 'visibilitychange',
		)
		expect(move).toBeDefined()
		expect(visibility).toBeDefined()
		unmount()
		expect(remove).toHaveBeenCalledWith('pointermove', move?.[1])
		expect(docRemove).toHaveBeenCalledWith('visibilitychange', visibility?.[1])
		expect(caf).toHaveBeenCalledWith(1)
	})
})
