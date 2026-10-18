import { Loader2, ShieldAlert } from 'lucide-react'
import { useEffect } from 'react'
import { useAuth } from 'react-oidc-context'
import { FullscreenMessage } from '#/components/layout/fullscreen-message'
import { Button } from '#/components/ui/button'
import { returnToState } from '#/lib/return-to'
import { getOidcConfiguration } from '#/lib/runtime-config'

interface Props {
	children: React.ReactNode
}

export function AuthGate({ children }: Props) {
	const auth = useAuth()
	const isConfigured = Boolean(getOidcConfiguration())

	useEffect(() => {
		if (!isConfigured || !auth) return
		if (auth.isLoading || auth.activeNavigator) return
		if (!auth.isAuthenticated && !auth.error) {
			void auth.signinRedirect({ state: returnToState(window.location) })
		}
	}, [auth, isConfigured])

	if (!isConfigured || !auth) {
		return (
			<FullscreenMessage
				icon={<ShieldAlert aria-hidden="true" className="size-5" />}
				title="Authentification non configurée"
				message="Définissez VITE_OIDC_ISSUER_URL et VITE_OIDC_CLIENT_ID (dev) ou issuer_url et client_id dans /config.json (prod)."
			/>
		)
	}

	if (auth.error) {
		return (
			<FullscreenMessage
				icon={
					<ShieldAlert
						aria-hidden="true"
						className="size-5 text-signal-danger"
					/>
				}
				title="Erreur d'authentification"
				message={auth.error.message}
				action={
					<Button
						onClick={() => {
							void auth.removeUser().then(() =>
								auth.signinRedirect({
									state: returnToState(window.location),
								}),
							)
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
				icon={<Loader2 aria-hidden="true" className="size-5 animate-spin" />}
				title="Redirection vers le fournisseur d'identité…"
				message="Vous allez être redirigé pour vous connecter"
			/>
		)
	}

	return <>{children}</>
}
