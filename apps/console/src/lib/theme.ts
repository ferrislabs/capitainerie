import { Store } from '@tanstack/store'

export type ThemePreference = 'light' | 'dark' | 'system'
export type ResolvedTheme = 'light' | 'dark'

export const THEME_STORAGE_KEY = 'capitainerie.theme'
export const THEME_PREFERENCES: ReadonlyArray<ThemePreference> = [
	'light',
	'dark',
	'system',
]

const DARK_QUERY = '(prefers-color-scheme: dark)'

export function parseThemePreference(value: unknown): ThemePreference {
	return value === 'light' || value === 'dark' || value === 'system'
		? value
		: 'system'
}

export function readThemePreference(): ThemePreference {
	try {
		return parseThemePreference(window.localStorage.getItem(THEME_STORAGE_KEY))
	} catch {
		return 'system'
	}
}

export function systemPrefersDark(): boolean {
	return (
		typeof window.matchMedia === 'function' &&
		window.matchMedia(DARK_QUERY).matches
	)
}

export function resolveTheme(
	preference: ThemePreference,
	prefersDark: boolean,
): ResolvedTheme {
	if (preference === 'system') return prefersDark ? 'dark' : 'light'
	return preference
}

export function applyTheme(theme: ResolvedTheme, root: HTMLElement): void {
	root.classList.toggle('dark', theme === 'dark')
	root.style.colorScheme = theme
}

export const themeStore = new Store<{ preference: ThemePreference }>({
	preference: 'system',
})

export function setThemePreference(preference: ThemePreference): void {
	try {
		window.localStorage.setItem(THEME_STORAGE_KEY, preference)
	} catch {
		// Stockage indisponible (navigation privée) : le choix vaut pour la session.
	}
	themeStore.setState(() => ({ preference }))
	applyTheme(
		resolveTheme(preference, systemPrefersDark()),
		document.documentElement,
	)
}

export function startThemeSync(): () => void {
	const initial = readThemePreference()
	themeStore.setState(() => ({ preference: initial }))
	applyTheme(
		resolveTheme(initial, systemPrefersDark()),
		document.documentElement,
	)

	if (typeof window.matchMedia !== 'function') return () => {}
	const media = window.matchMedia(DARK_QUERY)
	const onChange = () => {
		applyTheme(
			resolveTheme(themeStore.state.preference, media.matches),
			document.documentElement,
		)
	}
	media.addEventListener('change', onChange)
	return () => media.removeEventListener('change', onChange)
}

export const THEME_BOOT_SCRIPT = `(function(){try{var p=localStorage.getItem('${THEME_STORAGE_KEY}');var d=p==='dark'||((p!=='light')&&window.matchMedia('${DARK_QUERY}').matches);var r=document.documentElement;r.classList.toggle('dark',d);r.style.colorScheme=d?'dark':'light'}catch(e){}})()`
