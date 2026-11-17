import type { LucideIcon } from 'lucide-react'
import { LogOut, Monitor, Moon, Sun } from 'lucide-react'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuRadioGroup,
	DropdownMenuRadioItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '#/components/ui/dropdown-menu'
import { HumanAvatar } from '#/domain/dashboard/ui/avatars'
import { UserBadge } from '#/domain/identity/ui/user-badge'
import { parseThemePreference, type ThemePreference } from '#/lib/theme'
import { cn } from '#/lib/utils'

const THEMES: ReadonlyArray<{
	value: ThemePreference
	label: string
	icon: LucideIcon
}> = [
	{ value: 'light', label: 'Clair', icon: Sun },
	{ value: 'dark', label: 'Sombre', icon: Moon },
	{ value: 'system', label: 'Comme le système', icon: Monitor },
]

interface Props {
	displayName: string
	email?: string
	organizationName: string
	label?: string
	compact?: boolean
	theme: ThemePreference
	onThemeChange: (theme: ThemePreference) => void
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
	label = 'Board',
	compact = false,
	theme,
	onThemeChange,
	onSignOut,
}: Props) {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger
				aria-label={`Compte de ${displayName}`}
				title={compact ? displayName : undefined}
				className={cn(
					'flex h-7 min-w-0 items-center gap-2 rounded-[6px] text-left text-muted-foreground transition-colors duration-150 data-[state=open]:bg-secondary',
					'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring',
					compact ? 'justify-center' : 'flex-1',
				)}
			>
				<HumanAvatar initials="BO" size={22} />
				{compact ? null : (
					<span className="min-w-0 flex-1 truncate text-xs">{label}</span>
				)}
			</DropdownMenuTrigger>
			<DropdownMenuContent
				side={compact ? 'right' : 'top'}
				align={compact ? 'end' : 'start'}
				className="w-64"
			>
				<DropdownMenuLabel>
					<UserBadge
						displayName={displayName}
						email={email}
						organizationName={organizationName}
					/>
				</DropdownMenuLabel>
				<DropdownMenuSeparator />
				<DropdownMenuLabel className="pb-1 text-xs font-normal text-muted-foreground">
					Thème
				</DropdownMenuLabel>
				<DropdownMenuRadioGroup
					value={theme}
					onValueChange={(value) => onThemeChange(parseThemePreference(value))}
				>
					{THEMES.map(({ value, label, icon: Icon }) => (
						<DropdownMenuRadioItem key={value} value={value}>
							<Icon aria-hidden="true" />
							{label}
						</DropdownMenuRadioItem>
					))}
				</DropdownMenuRadioGroup>
				<DropdownMenuSeparator />
				<DropdownMenuItem onSelect={onSignOut}>
					<LogOut aria-hidden="true" />
					Se déconnecter
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	)
}
