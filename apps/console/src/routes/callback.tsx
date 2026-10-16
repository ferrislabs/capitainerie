import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { Loader2, ShieldAlert } from 'lucide-react'
import { useEffect } from 'react'
import { useAuth } from 'react-oidc-context'
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
			<div className="flex min-h-screen flex-col items-center justify-center gap-3 p-8 text-center">
				<ShieldAlert className="size-8 text-destructive" />
				<p className="font-medium">Connexion impossible</p>
				<p className="text-sm text-muted-foreground">{auth.error.message}</p>
				<Button onClick={() => void navigate({ to: '/', replace: true })}>
					Réessayer
				</Button>
			</div>
		)
	}

	return (
		<div className="flex min-h-screen items-center justify-center">
			<Loader2 className="size-6 animate-spin text-primary" />
		</div>
	)
}
