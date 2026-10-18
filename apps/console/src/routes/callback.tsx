import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { Loader2, ShieldAlert } from 'lucide-react'
import { useEffect } from 'react'
import { useAuth } from 'react-oidc-context'
import { FullscreenMessage } from '#/components/layout/fullscreen-message'
import { Button } from '#/components/ui/button'
import { safeReturnTo } from '#/lib/return-to'

export const Route = createFileRoute('/callback')({ component: Callback })

function Callback() {
	const auth = useAuth()
	const navigate = useNavigate()

	useEffect(() => {
		if (auth.isLoading || !auth.isAuthenticated) return
		void navigate({ to: safeReturnTo(auth.user?.state), replace: true })
	}, [auth.isLoading, auth.isAuthenticated, auth.user, navigate])

	if (auth.error) {
		return (
			<FullscreenMessage
				icon={
					<ShieldAlert
						aria-hidden="true"
						className="size-5 text-signal-danger"
					/>
				}
				title="Connexion impossible"
				message={auth.error.message}
				action={
					<Button onClick={() => void navigate({ to: '/', replace: true })}>
						Réessayer
					</Button>
				}
			/>
		)
	}

	return (
		<FullscreenMessage
			icon={<Loader2 aria-hidden="true" className="size-5 animate-spin" />}
			title="Connexion en cours…"
		/>
	)
}
