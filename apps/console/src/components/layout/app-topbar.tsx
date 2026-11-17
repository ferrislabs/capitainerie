import { Link } from '@tanstack/react-router'
import { Moon, Search, Sun } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Crumb } from '#/components/layout/breadcrumbs'
import { cn } from '#/lib/utils'

interface Props {
	crumbs: ReadonlyArray<Crumb>
	onSearch: () => void
	dark: boolean
	onToggleTheme: () => void
	trailing?: ReactNode
}

const FOCUS =
	'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring'

export function AppTopbar({
	crumbs,
	onSearch,
	dark,
	onToggleTheme,
	trailing,
}: Props) {
	const ThemeIcon = dark ? Sun : Moon
	const themeLabel = dark ? 'Passer au thème clair' : 'Passer au thème sombre'
	return (
		<header className="flex h-10 shrink-0 items-center gap-3 border-b bg-background px-4">
			<nav aria-label="Fil d'Ariane" className="min-w-0 flex-1">
				<ol className="flex min-w-0 items-center gap-1.5 text-muted-foreground">
					{crumbs.map((crumb, index) => {
						const last = index === crumbs.length - 1
						return (
							<li
								key={crumb.to ?? crumb.label}
								className="flex min-w-0 items-center gap-1.5"
							>
								{index > 0 ? <span aria-hidden="true">/</span> : null}
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
									<span
										aria-current={last ? 'page' : undefined}
										className={cn(
											'truncate',
											last && 'font-medium text-foreground',
										)}
									>
										{crumb.label}
									</span>
								)}
							</li>
						)
					})}
				</ol>
			</nav>
			{trailing}
			<button
				type="button"
				onClick={onSearch}
				aria-label="Rechercher"
				className={cn(
					'hidden h-7 w-56 items-center gap-2 rounded-md border bg-card px-2 text-muted-foreground hover:text-foreground sm:flex',
					FOCUS,
				)}
			>
				<Search aria-hidden="true" className="size-3.5" />
				<span className="flex-1 text-left">Rechercher…</span>
				<kbd className="font-mono text-[11px]">⌘K</kbd>
			</button>
			<button
				type="button"
				onClick={onToggleTheme}
				aria-label={themeLabel}
				title={themeLabel}
				className={cn(
					'flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground',
					FOCUS,
				)}
			>
				<ThemeIcon aria-hidden="true" className="size-4" />
			</button>
		</header>
	)
}
