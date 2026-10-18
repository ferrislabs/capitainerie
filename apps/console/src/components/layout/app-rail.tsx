import { Link } from '@tanstack/react-router'
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import type { ReactNode } from 'react'
import {
	CapitainerieMark,
	CapitainerieWordmark,
} from '#/components/brand/capitainerie-mark'
import type { NavItem, NavSection } from '#/components/layout/nav-items'
import { moveFocusBetweenLinks } from '#/components/layout/roving-focus'
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from '#/components/ui/tooltip'
import { cn } from '#/lib/utils'

interface Props {
	sections: ReadonlyArray<NavSection>
	expanded: boolean
	canExpand: boolean
	onExpandedChange: (expanded: boolean) => void
	footer?: (expanded: boolean) => ReactNode
}

const FOCUS =
	'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring'

function RailLink({ item, expanded }: { item: NavItem; expanded: boolean }) {
	const Icon = item.icon
	const link = (
		<Link
			to={item.to}
			aria-label={expanded ? undefined : item.label}
			className={cn(
				'flex h-8 items-center gap-2.5 rounded-md px-2 text-sm text-rail-foreground transition-colors duration-150',
				'hover:bg-rail-accent/60 hover:text-rail-foreground-active',
				'aria-[current=page]:bg-rail-accent aria-[current=page]:font-medium aria-[current=page]:text-rail-foreground-active',
				!expanded && 'justify-center px-0',
				FOCUS,
			)}
		>
			<Icon aria-hidden="true" className="size-4 shrink-0" />
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
	sections,
	expanded,
	canExpand,
	onExpandedChange,
	footer,
}: Props) {
	const isExpanded = expanded && canExpand
	const ToggleIcon = isExpanded ? PanelLeftClose : PanelLeftOpen
	const toggleLabel = isExpanded ? 'Réduire le menu' : 'Déplier le menu'

	return (
		<aside
			data-expanded={isExpanded}
			className={cn(
				'sticky top-0 flex h-dvh shrink-0 flex-col border-r border-rail-border bg-rail text-rail-foreground transition-[width] duration-200 ease-out-expo',
				isExpanded ? 'w-60' : 'w-14',
			)}
		>
			<div
				className={cn(
					'flex shrink-0 gap-1 px-2 pt-3 pb-2',
					isExpanded ? 'h-14 items-center' : 'flex-col items-center',
				)}
			>
				<Link
					to="/runs"
					aria-label="Capitainerie, accueil"
					className={cn(
						'flex h-9 min-w-0 flex-1 items-center gap-2 rounded-md px-1.5 text-rail-foreground-active',
						!isExpanded && 'flex-none justify-center px-0',
						FOCUS,
					)}
				>
					<CapitainerieMark className="size-7 shrink-0" />
					{isExpanded ? <CapitainerieWordmark /> : null}
				</Link>
				{canExpand ? (
					<button
						type="button"
						onClick={() => onExpandedChange(!isExpanded)}
						aria-label={toggleLabel}
						aria-expanded={isExpanded}
						title={toggleLabel}
						className={cn(
							'flex size-8 shrink-0 items-center justify-center rounded-md text-rail-label transition-colors duration-150 hover:bg-rail-accent/60 hover:text-rail-foreground-active',
							FOCUS,
						)}
					>
						<ToggleIcon aria-hidden="true" className="size-4" />
					</button>
				) : null}
			</div>
			<nav
				aria-label="Navigation principale"
				className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-3"
				onKeyDown={moveFocusBetweenLinks}
			>
				{sections.map((section, index) => {
					const labelId = `rail-section-${index}`
					return (
						<div
							key={section.label}
							className={cn(
								index > 0 &&
									(isExpanded
										? 'mt-5'
										: 'mt-2 border-t border-rail-border pt-2'),
							)}
						>
							<p
								id={labelId}
								className={cn(
									'px-2 pb-1.5 text-xs text-rail-label',
									!isExpanded && 'sr-only',
								)}
							>
								{section.label}
							</p>
							<ul aria-labelledby={labelId} className="flex flex-col gap-0.5">
								{section.items.map((item) => (
									<li key={item.to}>
										<RailLink item={item} expanded={isExpanded} />
									</li>
								))}
							</ul>
						</div>
					)
				})}
			</nav>
			{footer ? (
				<div className="flex shrink-0 flex-col gap-1 border-t border-rail-border p-2">
					{footer(isExpanded)}
				</div>
			) : null}
		</aside>
	)
}
