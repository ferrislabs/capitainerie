import { useCallback, useEffect, useState } from 'react'
import { cn } from '#/lib/utils'

/** Message de retour après une action, effacé tout seul. Remplace les toasts des maquettes. */
export function useNotice(
	durationMs = 4000,
): [string | null, (message: string) => void] {
	const [message, setMessage] = useState<string | null>(null)
	useEffect(() => {
		if (message === null) return
		const timer = setTimeout(() => setMessage(null), durationMs)
		return () => clearTimeout(timer)
	}, [message, durationMs])
	const notify = useCallback((next: string) => setMessage(next), [])
	return [message, notify]
}

interface Props {
	message: string | null | undefined
	className?: string
}

/** Zone vive annoncée aux lecteurs d'écran ; invisible tant qu'il n'y a rien à dire. */
export function Notice({ message, className }: Props) {
	return (
		<output
			aria-live="polite"
			className={cn(
				message
					? 'block rounded-lg border bg-card px-3 py-2 text-[13px] shadow-sm'
					: 'sr-only',
				className,
			)}
		>
			{message}
		</output>
	)
}
