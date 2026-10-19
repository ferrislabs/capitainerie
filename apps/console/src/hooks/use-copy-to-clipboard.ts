import { useCallback, useEffect, useRef, useState } from 'react'

const RESET_AFTER_MS = 2000

export function useCopyToClipboard(): {
	copied: boolean
	copy: (text: string) => void
	reset: () => void
} {
	const [copied, setCopied] = useState(false)
	const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

	const clear = useCallback(() => {
		if (timer.current !== null) {
			clearTimeout(timer.current)
			timer.current = null
		}
	}, [])

	useEffect(() => clear, [clear])

	const reset = useCallback(() => {
		clear()
		setCopied(false)
	}, [clear])

	const copy = useCallback(
		(text: string) => {
			navigator.clipboard
				.writeText(text)
				.then(() => {
					clear()
					setCopied(true)
					timer.current = setTimeout(() => setCopied(false), RESET_AFTER_MS)
				})
				.catch(() => setCopied(false))
		},
		[clear],
	)

	return { copied, copy, reset }
}
