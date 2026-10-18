import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
	parseThemePreference,
	resolveTheme,
	setThemePreference,
	startThemeSync,
	THEME_STORAGE_KEY,
	themeStore,
} from '#/lib/theme'

describe('theme', () => {
	beforeEach(() => {
		window.localStorage.clear()
		document.documentElement.className = ''
	})
	afterEach(() => {
		window.localStorage.clear()
	})

	it('falls back to system for unknown stored values', () => {
		expect(parseThemePreference('dark')).toBe('dark')
		expect(parseThemePreference('sepia')).toBe('system')
		expect(parseThemePreference(null)).toBe('system')
	})

	it('resolves system against the OS preference', () => {
		expect(resolveTheme('system', true)).toBe('dark')
		expect(resolveTheme('system', false)).toBe('light')
		expect(resolveTheme('light', true)).toBe('light')
	})

	it('persists the choice in localStorage and applies the dark class', () => {
		setThemePreference('dark')

		expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark')
		expect(themeStore.state.preference).toBe('dark')
		expect(document.documentElement.classList.contains('dark')).toBe(true)

		setThemePreference('light')

		expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe('light')
		expect(document.documentElement.classList.contains('dark')).toBe(false)
	})

	it('restores the stored choice on start', () => {
		window.localStorage.setItem(THEME_STORAGE_KEY, 'dark')

		const stop = startThemeSync()

		expect(themeStore.state.preference).toBe('dark')
		expect(document.documentElement.classList.contains('dark')).toBe(true)
		stop()
	})
})
