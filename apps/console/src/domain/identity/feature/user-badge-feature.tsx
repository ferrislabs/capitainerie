import { useMe } from '#/domain/identity/hooks/use-me'
import { UserBadge } from '#/domain/identity/ui/user-badge'

export function UserBadgeFeature() {
	const { data, isPending, isError } = useMe()

	if (isError) {
		return (
			<p role="alert" className="text-xs text-destructive">
				Impossible de charger le profil.
			</p>
		)
	}

	if (isPending || !data) {
		return <p className="text-xs text-muted-foreground">Chargement…</p>
	}

	return (
		<UserBadge
			displayName={data.display_name ?? data.username ?? data.subject}
			email={data.email ?? undefined}
			organizationName={data.organization.name}
		/>
	)
}
