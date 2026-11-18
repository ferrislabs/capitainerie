import { Link } from '@tanstack/react-router'
import { PanelLeft, Search, SquarePen, Users } from 'lucide-react'
import type { ReactNode } from 'react'
import type { NavItem, NavSection } from '#/components/layout/nav-items'
import { moveFocusBetweenLinks } from '#/components/layout/roving-focus'
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from '#/components/ui/tooltip'
import { AgentFace } from '#/domain/dashboard/ui/avatars'
import { LiveLabel } from '#/domain/dashboard/ui/live-label'
import { cn } from '#/lib/utils'

export type RailBadges = Readonly<Partial<Record<NavItem['to'], number>>>

export interface RailAgent {
	id: string
	name: string
	hue: number
}

interface Props {
	sections: ReadonlyArray<NavSection>
	expanded: boolean
	canExpand: boolean
	onExpandedChange: (expanded: boolean) => void
	footer?: (expanded: boolean) => ReactNode
	badges?: RailBadges
	onSearch?: () => void
	onNewTask?: () => void
	organization?: string
	liveCount?: number
	liveAgents?: ReadonlyArray<RailAgent>
}

const FOCUS =
	'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring'

const NO_CURRENT =
	'aria-[current=page]:bg-transparent aria-[current=page]:before:hidden hover:aria-[current=page]:bg-secondary'

const NAV =
	'group relative flex h-7 w-full items-center gap-[9px] rounded-[6px] px-2 text-left whitespace-nowrap text-muted-foreground transition-colors duration-150 hover:bg-secondary hover:text-foreground aria-[current=page]:bg-secondary aria-[current=page]:text-foreground aria-[current=page]:before:absolute aria-[current=page]:before:inset-y-1.5 aria-[current=page]:before:-left-1.5 aria-[current=page]:before:w-0.5 aria-[current=page]:before:rounded-sm aria-[current=page]:before:bg-primary'

const GROUP =
	'px-2 pt-3 pb-1 font-mono text-[10px] leading-none font-medium tracking-[0.09em] whitespace-nowrap text-faint uppercase'

function Dot() {
	return (
		<span
			aria-hidden="true"
			className="absolute top-1 right-[5px] size-1.5 rounded-full bg-primary"
		/>
	)
}

function RailLink({
	item,
	expanded,
	count,
	liveCount,
}: {
	item: NavItem
	expanded: boolean
	count: number | undefined
	liveCount: number
}) {
	const Icon = item.icon
	const live = item.live === true && liveCount > 0
	const shown = count !== undefined && count >= 100 ? '100+' : count
	const link = (
		<Link
			to={item.to}
			activeOptions={{ exact: item.to === '/' }}
			aria-label={expanded ? undefined : item.label}
			className={cn(NAV, !expanded && 'justify-center px-0', FOCUS)}
		>
			<Icon aria-hidden="true" className="size-4 shrink-0" />
			{expanded ? (
				<>
					<span className="flex-1 truncate">{item.label}</span>
					{live ? <LiveLabel>{`${liveCount} en direct`}</LiveLabel> : null}
					{shown !== undefined ? (
						<span className="figures rounded-lg bg-secondary px-1.5 text-[11px] leading-4 text-foreground group-aria-[current=page]:bg-border">
							{shown}
						</span>
					) : null}
				</>
			) : live || (count !== undefined && count > 0) ? (
				<Dot />
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

function ActionButton({
	label,
	icon: Icon,
	expanded,
	onClick,
}: {
	label: string
	icon: typeof Search
	expanded: boolean
	onClick: () => void
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			aria-label={expanded ? undefined : label}
			title={label}
			className={cn(NAV, !expanded && 'justify-center px-0', FOCUS)}
		>
			<Icon aria-hidden="true" className="size-4 shrink-0" />
			{expanded ? <span className="flex-1 truncate">{label}</span> : null}
		</button>
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
	organization,
	liveCount = 0,
	liveAgents,
}: Props) {
	const isExpanded = expanded && canExpand
	const toggleLabel = isExpanded ? 'Réduire le menu' : 'Déplier le menu'
	const agents = liveAgents ?? []

	return (
		<aside
			data-expanded={isExpanded}
			className={cn(
				'sticky top-0 flex h-dvh shrink-0 flex-col gap-0.5 overflow-x-hidden overflow-y-auto border-r bg-card px-1.5 py-2 transition-[width] duration-200 ease-out-expo',
				isExpanded ? 'w-[204px]' : 'w-12',
			)}
		>
			{organization ? (
				<div
					title={organization}
					className={cn(
						'mb-1.5 flex h-[34px] w-full items-center gap-2 rounded-md border bg-background px-1.5',
						!isExpanded && 'justify-center px-0',
					)}
				>
					<span
						aria-hidden="true"
						className="grid size-5 shrink-0 place-items-center rounded-[5px] bg-linear-to-br from-brand-from to-brand-to text-[11px] font-bold text-white"
					>
						{organization.charAt(0).toUpperCase()}
					</span>
					<b
						className={cn(
							'flex-1 truncate font-semibold whitespace-nowrap',
							!isExpanded && 'sr-only',
						)}
					>
						{organization}
					</b>
				</div>
			) : null}
			{onNewTask ? (
				<ActionButton
					label="Nouvelle tâche"
					icon={SquarePen}
					expanded={isExpanded}
					onClick={onNewTask}
				/>
			) : null}
			{onSearch ? (
				<ActionButton
					label="Rechercher"
					icon={Search}
					expanded={isExpanded}
					onClick={onSearch}
				/>
			) : null}
			<nav
				aria-label="Navigation principale"
				className="flex flex-1 flex-col gap-0.5"
				onKeyDown={moveFocusBetweenLinks}
			>
				{sections.map((section, index) => {
					const labelId = `rail-section-${index}`
					return (
						<div key={section.label} className="flex flex-col gap-0.5">
							{section.label ? (
								isExpanded ? (
									<p id={labelId} className={GROUP}>
										{section.label}
									</p>
								) : (
									<>
										<p id={labelId} className="sr-only">
											{section.label}
										</p>
										<div
											aria-hidden="true"
											className="mx-1.5 my-2 h-px bg-border"
										/>
									</>
								)
							) : null}
							<ul
								aria-labelledby={section.label ? labelId : undefined}
								aria-label={section.label ? undefined : 'Général'}
								className="flex flex-col gap-0.5"
							>
								{section.items.map((item) => (
									<li key={item.to}>
										<RailLink
											item={item}
											expanded={isExpanded}
											count={badges?.[item.to]}
											liveCount={liveCount}
										/>
									</li>
								))}
							</ul>
						</div>
					)
				})}
				<div className="flex flex-col gap-0.5">
					{isExpanded ? (
						<p className={GROUP}>Agents</p>
					) : (
						<div aria-hidden="true" className="mx-1.5 my-2 h-px bg-border" />
					)}
					<ul aria-label="Agents en direct" className="flex flex-col gap-0.5">
						{agents.map((agent) => (
							<li key={agent.id}>
								<Link
									to="/agents/$agentId/{-$tab}"
									params={{ agentId: agent.id }}
									aria-label={isExpanded ? undefined : agent.name}
									title={agent.name}
									className={cn(
										NAV,
										NO_CURRENT,
										!isExpanded && 'justify-center px-0',
										FOCUS,
									)}
								>
									<AgentFace hue={agent.hue} size={18} />
									{isExpanded ? (
										<>
											<span className="flex-1 truncate">{agent.name}</span>
											<LiveLabel>en direct</LiveLabel>
										</>
									) : null}
								</Link>
							</li>
						))}
						<li>
							<Link
								to="/agents"
								aria-label={isExpanded ? undefined : 'Voir tous les agents'}
								title="Tous les agents"
								className={cn(
									NAV,
									NO_CURRENT,
									!isExpanded && 'justify-center px-0',
									FOCUS,
								)}
							>
								<Users aria-hidden="true" className="size-4 shrink-0" />
								{isExpanded ? (
									<span className="flex-1 truncate text-faint">
										Voir tous les agents
									</span>
								) : null}
							</Link>
						</li>
					</ul>
				</div>
			</nav>
			<div
				className={cn(
					'mt-1.5 flex gap-2 border-t px-1 pt-1.5 text-muted-foreground',
					isExpanded ? 'items-center' : 'flex-col items-center',
				)}
			>
				{footer ? footer(isExpanded) : null}
				{canExpand ? (
					<button
						type="button"
						onClick={() => onExpandedChange(!isExpanded)}
						aria-label={toggleLabel}
						aria-expanded={isExpanded}
						title={toggleLabel}
						className={cn(
							'grid size-7 shrink-0 place-items-center rounded-[6px] text-muted-foreground hover:bg-secondary hover:text-foreground',
							FOCUS,
						)}
					>
						<PanelLeft aria-hidden="true" className="size-4" />
					</button>
				) : null}
			</div>
		</aside>
	)
}
