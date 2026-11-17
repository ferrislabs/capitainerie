import { useEffect, useState } from 'react'

/** État d'ouverture de la palette, basculé par ⌘K ou Ctrl+K. */
export function useCommandPalette() {
	const [open, setOpen] = useState(false)
	useEffect(() => {
		function onKeyDown(event: KeyboardEvent) {
			if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
				event.preventDefault()
				setOpen((value) => !value)
			}
		}
		document.addEventListener('keydown', onKeyDown)
		return () => document.removeEventListener('keydown', onKeyDown)
	}, [])
	return { open, setOpen }
}
