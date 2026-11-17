import { Link, type LinkProps } from '@tanstack/react-router'
import { cn } from '#/lib/utils'

export interface TabLink {
	label: string
	link: LinkProps
}

interface Props {
	label: string
	tabs: ReadonlyArray<TabLink>
	className?: string
}

/** Onglets portés par l'URL : chaque onglet est un lien, l'actif porte `aria-current="page"`. */
export function TabNav({ label, tabs, className }: Props) {
	return (
		<nav
			aria-label={label}
			className={cn('flex gap-0.5 overflow-x-auto border-b', className)}
		>
			{tabs.map((tab) => (
				<Link
					key={tab.label}
					{...tab.link}
					className={cn(
						'-mb-px whitespace-nowrap border-b-2 border-transparent px-2.5 py-1.5 font-medium text-muted-foreground hover:text-foreground',
						'aria-[current=page]:border-primary aria-[current=page]:text-foreground',
						'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring',
					)}
				>
					{tab.label}
				</Link>
			))}
		</nav>
	)
}
