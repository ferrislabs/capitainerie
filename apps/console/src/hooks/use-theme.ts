import { useEffect, useSyncExternalStore } from 'react'
import {
	setThemePreference,
	startThemeSync,
	type ThemePreference,
	themeStore,
} from '#/lib/theme'

export function useThemeSync(): void {
	useEffect(() => startThemeSync(), [])
}

export function useTheme(): {
	preference: ThemePreference
	setPreference: (preference: ThemePreference) => void
} {
	const preference = useSyncExternalStore(
		(onChange) => themeStore.subscribe(onChange).unsubscribe,
		() => themeStore.state.preference,
		() => 'system' as const,
	)
	return { preference, setPreference: setThemePreference }
}
