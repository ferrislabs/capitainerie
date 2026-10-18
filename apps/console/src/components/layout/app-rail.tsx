import { Link } from '@tanstack/react-router'
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import {
	CapitainerieMark,
	CapitainerieWordmark,
} from '#/components/brand/capitainerie-mark'
import type { NavItem } from '#/components/layout/nav-items'
import { moveFocusBetweenLinks } from '#/components/layout/roving-focus'
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from '#/components/ui/tooltip'
import { cn } from '#/lib/utils'

interface Props {
	primary: ReadonlyArray<NavItem>
	secondary: ReadonlyArray<NavItem>
	expanded: boolean
	canExpand: boolean
	onExpandedChange: (expanded: boolean) => void
}

function RailLink({ item, expanded }: { item: NavItem; expanded: boolean }) {
	const Icon = item.icon
	const link = (
		<Link
			to={item.to}
			aria-label={expanded ? undefined : item.label}
			className={cn(
				'group relative flex h-9 items-center gap-3 rounded-md px-2.5 text-[13px] font-medium text-rail-foreground transition-colors duration-150',
				'hover:bg-rail-accent/70 hover:text-rail-foreground-active',
				'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-rail-indicator',
				'aria-[current=page]:bg-rail-accent aria-[current=page]:text-rail-foreground-active',
			)}
		>
			<span
				aria-hidden="true"
				className="absolute top-2 bottom-2 -left-2 w-[3px] rounded-r-full bg-rail-indicator opacity-0 transition-opacity duration-150 group-aria-[current=page]:opacity-100"
			/>
			<Icon aria-hidden="true" className="size-[18px] shrink-0" />
			{expanded ? <span className="truncate">{item.label}</span> : null}
		</Link>
	)

	if (expanded) return link
	return (
		<Tooltip>
			<TooltipTrigger asChild>{link}</TooltipTrigger>
			<TooltipContent side="right">{item.label}</TooltipContent>
		</Tooltip>
	)
}

export function AppRail({
	primary,
	secondary,
	expanded,
	canExpand,
	onExpandedChange,
}: Props) {
	const isExpanded = expanded && canExpand
	const ToggleIcon = isExpanded ? PanelLeftClose : PanelLeftOpen
	const toggleLabel = isExpanded ? 'Réduire le menu' : 'Déplier le menu'

	return (
		<aside
			data-expanded={isExpanded}
			className={cn(
				'sticky top-0 flex h-dvh shrink-0 flex-col border-r border-rail-border bg-rail px-2 py-3 text-rail-foreground transition-[width] duration-200 ease-out-expo',
				isExpanded ? 'w-52' : 'w-14',
			)}
		>
			<Link
				to="/runs"
				aria-label="Capitainerie, accueil"
				className="mb-4 flex h-9 items-center gap-2.5 rounded-md px-1 text-rail-foreground-active focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-rail-indicator"
			>
				<CapitainerieMark className="size-8 shrink-0" />
				{isExpanded ? <CapitainerieWordmark /> : null}
			</Link>
			<nav
				aria-label="Navigation principale"
				className="flex flex-1 flex-col"
				onKeyDown={moveFocusBetweenLinks}
			>
				<ul className="flex flex-col gap-0.5">
					{primary.map((item) => (
						<li key={item.to}>
							<RailLink item={item} expanded={isExpanded} />
						</li>
					))}
				</ul>
				<ul className="mt-auto flex flex-col gap-0.5 border-t border-rail-border pt-2">
					{secondary.map((item) => (
						<li key={item.to}>
							<RailLink item={item} expanded={isExpanded} />
						</li>
					))}
				</ul>
			</nav>
			{canExpand ? (
				<button
					type="button"
					onClick={() => onExpandedChange(!isExpanded)}
					aria-label={toggleLabel}
					aria-expanded={isExpanded}
					className="mt-2 flex h-8 items-center gap-3 rounded-md px-2.5 text-xs text-rail-foreground transition-colors hover:bg-rail-accent/70 hover:text-rail-foreground-active focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-rail-indicator"
				>
					<ToggleIcon aria-hidden="true" className="size-4 shrink-0" />
					{isExpanded ? <span>Réduire</span> : null}
				</button>
			) : null}
		</aside>
	)
}
