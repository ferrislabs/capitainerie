import { Link } from '@tanstack/react-router'
import { PanelLeftClose, PanelLeftOpen, Search, SquarePen } from 'lucide-react'
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

export type RailBadges = Readonly<Partial<Record<NavItem['to'], number>>>

interface Props {
	sections: ReadonlyArray<NavSection>
	expanded: boolean
	canExpand: boolean
	onExpandedChange: (expanded: boolean) => void
	footer?: (expanded: boolean) => ReactNode
	badges?: RailBadges
	onSearch?: () => void
	onNewTask?: () => void
	liveAgents?: ReadonlyArray<{ id: string; name: string }>
}

const FOCUS =
	'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring'

function RailLink({
	item,
	expanded,
	count,
}: {
	item: NavItem
	expanded: boolean
	count: number | undefined
}) {
	const Icon = item.icon
	const showCount = count !== undefined && count > 0
	const shown = count !== undefined && count >= 100 ? '100+' : count
	const link = (
		<Link
			to={item.to}
			activeOptions={{ exact: item.to === '/' }}
			aria-label={
				expanded
					? undefined
					: showCount
						? `${item.label}, ${shown} runs actifs`
						: item.label
			}
			className={cn(
				'flex h-7 items-center gap-2.5 rounded-md px-2 text-[13px] text-rail-foreground transition-colors duration-150',
				'hover:bg-rail-accent/60 hover:text-rail-foreground-active',
				'aria-[current=page]:bg-rail-accent aria-[current=page]:font-medium aria-[current=page]:text-rail-foreground-active',
				!expanded && 'justify-center px-0',
				FOCUS,
			)}
		>
			<span className="relative flex shrink-0">
				<Icon aria-hidden="true" className="size-4" />
				{showCount && !expanded ? (
					<span
						aria-hidden="true"
						className="absolute -top-1 -right-1 size-2 rounded-full bg-signal-running"
					/>
				) : null}
			</span>
			{expanded ? <span className="truncate">{item.label}</span> : null}
			{showCount && expanded ? (
				<>
					<span
						aria-hidden="true"
						className="figures ml-auto rounded-sm bg-signal-running-soft px-1.5 text-xs leading-5 font-medium text-signal-running"
					>
						{shown}
					</span>
					<span className="sr-only">{`, ${shown} runs actifs`}</span>
				</>
			) : null}
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
	badges,
	onSearch,
	onNewTask,
	liveAgents,
}: Props) {
	const isExpanded = expanded && canExpand
	const ToggleIcon = isExpanded ? PanelLeftClose : PanelLeftOpen
	const toggleLabel = isExpanded ? 'Réduire le menu' : 'Déplier le menu'

	return (
		<aside
			data-expanded={isExpanded}
			className={cn(
				'sticky top-0 flex h-dvh shrink-0 flex-col border-r border-rail-border bg-rail text-rail-foreground transition-[width] duration-200 ease-out-expo',
				isExpanded ? 'w-[204px]' : 'w-12',
			)}
		>
			<div
				className={cn(
					'flex shrink-0 gap-1 px-2 pt-2 pb-1',
					isExpanded ? 'h-11 items-center' : 'flex-col items-center',
				)}
			>
				<Link
					to="/"
					aria-label="Capitainerie, accueil"
					className={cn(
						'flex h-8 min-w-0 flex-1 items-center gap-2 rounded-md px-1.5 text-rail-foreground-active',
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
							'flex size-7 shrink-0 items-center justify-center rounded-md text-rail-label transition-colors duration-150 hover:bg-rail-accent/60 hover:text-rail-foreground-active',
							FOCUS,
						)}
					>
						<ToggleIcon aria-hidden="true" className="size-4" />
					</button>
				) : null}
			</div>
			{onSearch || onNewTask ? (
				<div className="flex shrink-0 flex-col gap-0.5 px-2 pb-2">
					{onNewTask ? (
						<button
							type="button"
							onClick={onNewTask}
							aria-label="Nouvelle tâche"
							title="Nouvelle tâche"
							className={cn(
								'flex h-7 w-full items-center gap-2.5 rounded-md px-2 text-[13px] text-rail-foreground hover:bg-rail-accent/60 hover:text-rail-foreground-active',
								!isExpanded && 'justify-center px-0',
								FOCUS,
							)}
						>
							<SquarePen aria-hidden="true" className="size-4 shrink-0" />
							{isExpanded ? (
								<span className="flex-1 text-left">Nouvelle tâche</span>
							) : null}
						</button>
					) : null}
					{onSearch ? (
						<button
							type="button"
							onClick={onSearch}
							aria-label="Rechercher"
							title="Rechercher (⌘K)"
							className={cn(
								'flex h-7 w-full items-center gap-2.5 rounded-md px-2 text-[13px] text-rail-foreground hover:bg-rail-accent/60 hover:text-rail-foreground-active',
								!isExpanded && 'justify-center px-0',
								FOCUS,
							)}
						>
							<Search aria-hidden="true" className="size-4 shrink-0" />
							{isExpanded ? (
								<>
									<span className="flex-1 text-left">Rechercher</span>
									<kbd className="font-mono text-[11px] text-rail-label">
										⌘K
									</kbd>
								</>
							) : null}
						</button>
					) : null}
				</div>
			) : null}
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
										? 'mt-4'
										: 'mt-2 border-t border-rail-border pt-2'),
							)}
						>
							<p
								id={labelId}
								className={cn(
									'px-2 pb-1 text-[10.5px] font-medium uppercase tracking-[0.08em] text-rail-label',
									!isExpanded && 'sr-only',
								)}
							>
								{section.label}
							</p>
							<ul aria-labelledby={labelId} className="flex flex-col gap-0.5">
								{section.items.map((item) => (
									<li key={item.to}>
										<RailLink
											item={item}
											expanded={isExpanded}
											count={badges?.[item.to]}
										/>
									</li>
								))}
							</ul>
						</div>
					)
				})}
				{isExpanded && liveAgents && liveAgents.length > 0 ? (
					<div className="mt-4">
						<p className="px-2 pb-1 text-[10.5px] font-medium uppercase tracking-[0.08em] text-rail-label">
							Agents en direct
						</p>
						<ul aria-label="Agents en direct" className="flex flex-col gap-0.5">
							{liveAgents.map((agent) => (
								<li key={agent.id}>
									<Link
										to="/agents/equipe/$agentId/{-$tab}"
										params={{ agentId: agent.id }}
										className={cn(
											'flex h-7 items-center gap-2.5 rounded-md px-2 text-[13px] text-rail-foreground transition-colors duration-150',
											'hover:bg-rail-accent/60 hover:text-rail-foreground-active',
											FOCUS,
										)}
									>
										<span
											aria-hidden="true"
											className="size-2 shrink-0 rounded-full bg-signal-running"
										/>
										<span className="truncate">{agent.name}</span>
									</Link>
								</li>
							))}
						</ul>
					</div>
				) : null}
			</nav>
			{footer ? (
				<div className="flex shrink-0 flex-col gap-1 border-t border-rail-border p-2">
					{footer(isExpanded)}
				</div>
			) : null}
		</aside>
	)
}
