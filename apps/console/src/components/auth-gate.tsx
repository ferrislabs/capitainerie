import { Loader2, ShieldAlert } from 'lucide-react'
import { useEffect } from 'react'
import { useAuth } from 'react-oidc-context'
import { Button } from '#/components/ui/button'
import { getOidcConfiguration } from '#/lib/runtime-config'

interface Props {
	children: React.ReactNode
}

interface MessageProps {
	icon: React.ReactNode
	title: string
	message: string
	action?: React.ReactNode
}

function FullscreenMessage({ icon, title, message, action }: MessageProps) {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center gap-3 p-8 text-center">
			<div className="flex size-14 items-center justify-center rounded-xl border bg-card">
				{icon}
			</div>
			<div>
				<p className="font-medium">{title}</p>
				<p className="mt-1 text-sm text-muted-foreground">{message}</p>
			</div>
			{action}
		</div>
	)
}

export function AuthGate({ children }: Props) {
	const auth = useAuth()
	const isConfigured = Boolean(getOidcConfiguration())

	useEffect(() => {
		if (!isConfigured || !auth) return
		if (auth.isLoading || auth.activeNavigator) return
		if (!auth.isAuthenticated && !auth.error) {
			void auth.signinRedirect()
		}
	}, [auth, isConfigured])

	if (!isConfigured || !auth) {
		return (
			<FullscreenMessage
				icon={<ShieldAlert className="size-8" />}
				title="Authentification non configurée"
				message="Définissez VITE_OIDC_ISSUER_URL et VITE_OIDC_CLIENT_ID (dev) ou issuer_url et client_id dans /config.json (prod)."
			/>
		)
	}

	if (auth.error) {
		return (
			<FullscreenMessage
				icon={<ShieldAlert className="size-8 text-destructive" />}
				title="Erreur d'authentification"
				message={auth.error.message}
				action={
					<Button
						onClick={() => {
							void auth.removeUser().then(() => auth.signinRedirect())
						}}
					>
						Se reconnecter
					</Button>
				}
			/>
		)
	}

	if (!auth.isAuthenticated) {
		return (
			<FullscreenMessage
				icon={<Loader2 className="size-8 animate-spin text-primary" />}
				title="Redirection vers le fournisseur d'identité…"
				message="Vous allez être redirigé pour vous connecter"
			/>
		)
	}

	return <>{children}</>
}
