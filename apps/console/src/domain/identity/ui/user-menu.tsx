import { ChevronDown, LogOut, UserRound } from 'lucide-react'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '#/components/ui/dropdown-menu'
import { UserBadge } from '#/domain/identity/ui/user-badge'

interface Props {
	displayName: string
	email?: string
	organizationName: string
	isPending?: boolean
	onSignOut: () => void
}

export function initials(name: string): string {
	const parts = name.trim().split(/\s+/).filter(Boolean)
	const letters =
		parts.length > 1
			? `${parts[0][0]}${parts[parts.length - 1][0]}`
			: (parts[0] ?? '?').slice(0, 2)
	return letters.toUpperCase()
}

export function UserMenu({
	displayName,
	email,
	organizationName,
	isPending = false,
	onSignOut,
}: Props) {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger
				aria-label={`Compte de ${displayName}`}
				className="flex h-8 items-center gap-1.5 rounded-md py-1 pr-1.5 pl-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring data-[state=open]:bg-muted"
			>
				<span className="flex size-6 items-center justify-center rounded-sm bg-primary text-[11px] font-semibold text-primary-foreground">
					{isPending ? (
						<UserRound aria-hidden="true" className="size-3.5" />
					) : (
						initials(displayName)
					)}
				</span>
				<ChevronDown aria-hidden="true" className="size-3.5" />
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="w-64">
				<DropdownMenuLabel>
					<UserBadge
						displayName={displayName}
						email={email}
						organizationName={organizationName}
					/>
				</DropdownMenuLabel>
				<DropdownMenuSeparator />
				<DropdownMenuItem onSelect={onSignOut}>
					<LogOut aria-hidden="true" />
					Se déconnecter
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	)
}
