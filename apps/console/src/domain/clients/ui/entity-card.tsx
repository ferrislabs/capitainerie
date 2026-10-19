import { Link } from '@tanstack/react-router'
import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { Badge } from '#/components/ui/badge'
import { cn } from '#/lib/utils'

type Target =
	| { to: '/clients/$clientId'; params: { clientId: string } }
	| { to: '/projects/$projectId'; params: { projectId: string } }

type Props = Target & {
	icon: LucideIcon
	name: string
	archived: boolean
	hint?: ReactNode
	facts: ReadonlyArray<{ label: string; value: number }>
}

export function EntityCard({
	icon: Icon,
	name,
	archived,
	hint,
	facts,
	...target
}: Props) {
	return (
		<Link
			{...target}
			className={cn(
				'flex min-w-0 flex-col gap-2 rounded-lg border bg-card p-4 text-card-foreground transition-colors duration-150',
				'hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring',
			)}
		>
			<span className="flex items-center gap-2 text-[13px] font-medium text-muted-foreground">
				<Icon aria-hidden="true" className="size-4 shrink-0" />
				<span className="truncate">{name}</span>
				{archived ? (
					<Badge variant="muted" className="ml-auto">
						Archivé
					</Badge>
				) : null}
			</span>
			<span className="flex flex-wrap gap-x-5 gap-y-1">
				{facts.map((fact) => (
					<span key={fact.label} className="flex items-baseline gap-1.5">
						<span className="figures text-2xl leading-tight font-semibold tracking-[-0.02em]">
							{fact.value}
						</span>
						<span className="text-[13px] text-muted-foreground">
							{fact.label}
						</span>
					</span>
				))}
			</span>
			{hint ? (
				<span className="truncate text-[13px] text-muted-foreground">
					{hint}
				</span>
			) : null}
		</Link>
	)
}
