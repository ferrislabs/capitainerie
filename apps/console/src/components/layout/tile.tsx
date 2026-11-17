import { Link, type LinkProps } from '@tanstack/react-router'
import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	label: string
	value: ReactNode
	hint?: ReactNode
	icon?: LucideIcon
	/** `warn` : tuile pleine ambre, pour ce qui demande une action. */
	tone?: 'default' | 'warn'
	/** Fait de la tuile un lien. */
	link?: LinkProps
}

/** Chiffre clé de la maquette : libellé 12 px et icône, valeur de 30 px, précision grise. */
export function Tile({
	label,
	value,
	hint,
	icon: Icon,
	tone = 'default',
	link,
}: Props) {
	const warn = tone === 'warn'
	const className = cn(
		'flex min-w-0 flex-col gap-0.5 rounded-lg border px-3 pt-[11px] pb-2.5 text-left',
		warn
			? 'border-transparent bg-signal-attention text-signal-attention-foreground'
			: 'bg-card',
		link && !warn && 'hover:bg-secondary',
		link &&
			'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
	)
	const body = (
		<>
			<span
				className={cn(
					'flex items-center gap-1.5 text-xs font-semibold',
					!warn && 'text-muted-foreground',
				)}
			>
				{label}
				{Icon ? <Icon aria-hidden="true" className="ml-auto size-4" /> : null}
			</span>
			<span className="figures text-[30px] leading-[1.15] font-semibold tracking-[-0.02em]">
				{value}
			</span>
			{hint ? (
				<span
					className={cn(
						'text-[11.5px]',
						warn ? 'opacity-80' : 'text-muted-foreground',
					)}
				>
					{hint}
				</span>
			) : null}
		</>
	)
	return link ? (
		<Link {...link} className={className}>
			{body}
		</Link>
	) : (
		<div className={className}>{body}</div>
	)
}
