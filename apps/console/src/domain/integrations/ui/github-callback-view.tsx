import { Link } from '@tanstack/react-router'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'

interface Props {
	errorMessage?: string
	pendingLabel?: string
}

export function GithubCallbackView({
	errorMessage,
	pendingLabel = 'Connexion à GitHub en cours…',
}: Props) {
	if (!errorMessage) {
		return (
			<output className="text-sm text-muted-foreground">{pendingLabel}</output>
		)
	}

	return (
		<div className="flex flex-col items-start gap-3">
			<InlineAlert tone="danger" className="w-full">
				{errorMessage}
			</InlineAlert>
			<Button asChild variant="outline" size="sm">
				<Link to="/settings/integrations">Retour aux intégrations</Link>
			</Button>
		</div>
	)
}
