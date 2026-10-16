interface Props {
	displayName: string
	email?: string
	organizationName: string
}

export function UserBadge({ displayName, email, organizationName }: Props) {
	return (
		<div className="min-w-0 text-sm">
			<p className="truncate font-medium">{displayName}</p>
			{email ? (
				<p className="truncate text-xs text-muted-foreground">{email}</p>
			) : null}
			<p className="truncate text-xs text-muted-foreground">
				{organizationName}
			</p>
		</div>
	)
}
