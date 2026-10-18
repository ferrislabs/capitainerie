import { useAuth } from 'react-oidc-context'
import { useMe } from '#/domain/identity/hooks/use-me'
import { UserMenu } from '#/domain/identity/ui/user-menu'
import { useTheme } from '#/hooks/use-theme'

interface Props {
	compact?: boolean
}

export function UserMenuFeature({ compact = false }: Props) {
	const auth = useAuth()
	const theme = useTheme()
	const { data } = useMe()

	const displayName = data
		? (data.display_name ?? data.username ?? data.subject)
		: 'Compte'

	return (
		<UserMenu
			displayName={displayName}
			email={data?.email ?? undefined}
			organizationName={data?.organization.name ?? ''}
			isPending={!data}
			compact={compact}
			theme={theme.preference}
			onThemeChange={theme.setPreference}
			onSignOut={() => {
				void auth.signoutRedirect()
			}}
		/>
	)
}
