import type { LucideIcon } from 'lucide-react'
import { ChevronsUpDown, LogOut, Monitor, Moon, Sun } from 'lucide-react'
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
import { Skeleton } from '#/components/ui/skeleton'
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
	isPending?: boolean
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
	isPending = false,
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
					'flex h-11 w-full min-w-0 items-center gap-2.5 rounded-md px-1.5 text-left text-rail-foreground transition-colors duration-150 hover:bg-rail-accent/60 data-[state=open]:bg-rail-accent',
					'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring',
					compact && 'justify-center px-0',
				)}
			>
				<span className="flex size-8 shrink-0 items-center justify-center rounded-md border border-rail-border bg-card text-[11px] font-semibold text-foreground">
					{isPending ? '…' : initials(displayName)}
				</span>
				{compact ? null : (
					<>
						<span className="flex min-w-0 flex-1 flex-col">
							{isPending ? (
								<>
									<Skeleton className="h-3 w-24" />
									<Skeleton className="mt-1.5 h-2.5 w-16" />
								</>
							) : (
								<>
									<span className="truncate text-sm font-medium text-rail-foreground-active">
										{organizationName || displayName}
									</span>
									<span className="truncate text-xs text-rail-label">
										{displayName}
									</span>
								</>
							)}
						</span>
						<ChevronsUpDown
							aria-hidden="true"
							className="size-4 shrink-0 text-rail-label"
						/>
					</>
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
