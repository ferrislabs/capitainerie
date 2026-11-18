import { Link } from '@tanstack/react-router'
import { Moon, RefreshCw, Search, Sun } from 'lucide-react'
import type { Crumb } from '#/components/layout/breadcrumbs'
import { LiveLabel } from '#/domain/dashboard/ui/live-label'
import { cn } from '#/lib/utils'

interface Props {
	crumbs: ReadonlyArray<Crumb>
	organization?: string
	liveCount?: number
	onSearch: () => void
	dark: boolean
	onToggleTheme: () => void
	onRefresh?: () => void
	showRefresh?: boolean
}

const FOCUS =
	'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring'

const ICON_BUTTON = cn(
	'grid size-7 shrink-0 place-items-center rounded-[6px] text-muted-foreground hover:bg-secondary hover:text-foreground',
	FOCUS,
)

export function AppTopbar({
	crumbs,
	organization,
	liveCount = 0,
	onSearch,
	dark,
	onToggleTheme,
	onRefresh,
	showRefresh = true,
}: Props) {
	const ThemeIcon = dark ? Sun : Moon
	const themeLabel = dark ? 'Passer au thème clair' : 'Passer au thème sombre'
	return (
		<header className="flex h-[46px] shrink-0 items-center gap-2.5 border-b bg-background px-3.5">
			<nav aria-label="Fil d'Ariane" className="min-w-0 flex-1">
				<ol className="m-0 flex min-w-0 list-none items-center gap-1.5 overflow-hidden p-0 whitespace-nowrap text-muted-foreground">
					{organization ? (
						<li className="flex items-center gap-1.5">
							<span
								aria-hidden="true"
								className="grid size-[18px] shrink-0 place-items-center rounded-[5px] bg-linear-to-br from-brand-from to-brand-to text-[10px] font-bold text-white"
							>
								{organization.charAt(0).toUpperCase()}
							</span>
							{organization}
						</li>
					) : null}
					{crumbs.map((crumb, index) => {
						const last = index === crumbs.length - 1
						return (
							<li
								key={crumb.to ?? crumb.label}
								className="flex min-w-0 items-center gap-1.5"
							>
								{organization || index > 0 ? (
									<span aria-hidden="true" className="text-faint">
										/
									</span>
								) : null}
								{crumb.to && !last ? (
									<Link
										to={crumb.to}
										className={cn(
											'truncate rounded-sm hover:text-foreground',
											FOCUS,
										)}
									>
										{crumb.label}
									</Link>
								) : (
									<b
										aria-current={last ? 'page' : undefined}
										className="truncate font-semibold text-foreground"
									>
										{crumb.label}
									</b>
								)}
								{crumb.tag ? (
									<span className="font-mono text-[11px] font-medium text-faint">
										{crumb.tag}
									</span>
								) : null}
							</li>
						)
					})}
				</ol>
			</nav>
			{liveCount > 0 ? (
				<LiveLabel className="mr-0.5">{`${liveCount} en direct`}</LiveLabel>
			) : null}
			<button
				type="button"
				onClick={onSearch}
				aria-label="Rechercher"
				className={cn(
					'hidden h-[30px] min-w-[220px] items-center gap-2 rounded-md border bg-card px-2.5 text-muted-foreground sm:flex',
					FOCUS,
				)}
			>
				<Search aria-hidden="true" className="size-4 shrink-0" />
				<span>Rechercher tâches, agents, runs…</span>
				<kbd className="ml-auto font-mono text-[10.5px] font-medium text-faint">
					⌘K
				</kbd>
			</button>
			<div id="topbar-actions" className="flex items-center gap-1" />
			<button
				type="button"
				onClick={onToggleTheme}
				aria-label={themeLabel}
				title={themeLabel}
				className={ICON_BUTTON}
			>
				<ThemeIcon aria-hidden="true" className="size-4" />
			</button>
			{showRefresh ? (
				<button
					type="button"
					onClick={onRefresh}
					aria-label="Actualiser"
					title="Actualiser"
					className={ICON_BUTTON}
				>
					<RefreshCw aria-hidden="true" className="size-4" />
				</button>
			) : null}
		</header>
	)
}
