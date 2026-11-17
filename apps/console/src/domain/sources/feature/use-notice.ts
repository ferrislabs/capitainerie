import { useCallback, useEffect, useState } from 'react'

export function useNotice(): [string | null, (message: string) => void] {
	const [message, setMessage] = useState<string | null>(null)
	useEffect(() => {
		if (message === null) return
		const timer = setTimeout(() => setMessage(null), 2200)
		return () => clearTimeout(timer)
	}, [message])
	const notify = useCallback((next: string) => setMessage(next), [])
	return [message, notify]
}
