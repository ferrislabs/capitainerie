import { useAuth } from 'react-oidc-context'
import { useMe } from '#/domain/identity/hooks/use-me'
import { OrganizationName } from '#/domain/identity/ui/organization-name'
import { UserMenu } from '#/domain/identity/ui/user-menu'

export function OrganizationNameFeature() {
	const { data, isPending, isError } = useMe()
	return (
		<OrganizationName
			name={data?.organization.name}
			isPending={isPending}
			isError={isError}
		/>
	)
}

export function UserMenuFeature() {
	const auth = useAuth()
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
			onSignOut={() => {
				void auth.signoutRedirect()
			}}
		/>
	)
}
