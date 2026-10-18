import { Link } from '@tanstack/react-router'
import type { LucideIcon } from 'lucide-react'
import { moveFocusBetweenLinks } from '#/components/layout/roving-focus'
import { cn } from '#/lib/utils'

export interface SettingsNavItem {
	to: '/settings/providers' | '/settings/models'
	label: string
	icon: LucideIcon
}

interface Props {
	title: string
	groupLabel: string
	items: ReadonlyArray<SettingsNavItem>
}

export function SettingsNav({ title, groupLabel, items }: Props) {
	return (
		<nav
			aria-label={title}
			onKeyDown={moveFocusBetweenLinks}
			className="border-b bg-card lg:sticky lg:top-12 lg:h-[calc(100dvh-3rem)] lg:w-52 lg:shrink-0 lg:border-r lg:border-b-0"
		>
			<p className="hidden px-4 pt-5 pb-1 text-[15px] font-semibold lg:block">
				{title}
			</p>
			<p className="hidden px-4 pt-3 pb-1.5 text-xs font-medium text-muted-foreground lg:block">
				{groupLabel}
			</p>
			<ul className="flex gap-1 overflow-x-auto px-3 py-2 lg:flex-col lg:gap-0.5 lg:px-2 lg:py-0">
				{items.map(({ to, label, icon: Icon }) => (
					<li key={to}>
						<Link
							to={to}
							className={cn(
								'relative flex h-8 items-center gap-2.5 rounded-md px-2.5 text-[13px] font-medium whitespace-nowrap text-muted-foreground transition-colors duration-150',
								'hover:bg-muted hover:text-foreground',
								'aria-[current=page]:bg-secondary aria-[current=page]:text-secondary-foreground',
							)}
						>
							<Icon aria-hidden="true" className="size-4 shrink-0" />
							{label}
						</Link>
					</li>
				))}
			</ul>
		</nav>
	)
}
