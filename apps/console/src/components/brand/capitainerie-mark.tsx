import { cn } from '#/lib/utils'

interface Props {
	className?: string
}

/**
 * Le môle (arc ouvert sur le large), son feu (point jaune) et la marée (vague).
 * Les couleurs suivent le contexte : `currentColor` pour le môle et la vague,
 * le jaune de signal pour le feu.
 */
export function CapitainerieMark({ className }: Props) {
	return (
		<svg
			viewBox="0 0 32 32"
			fill="none"
			className={cn('size-8', className)}
			aria-hidden="true"
		>
			<path
				d="M21.6 8.4A9.6 9.6 0 1 0 21.6 23.6"
				stroke="currentColor"
				strokeWidth="2.6"
				strokeLinecap="round"
			/>
			<path
				d="M10.6 17.2c1.6-1.5 3.2-1.5 4.8 0s3.2 1.5 4.8 0"
				stroke="currentColor"
				strokeWidth="2.2"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			<circle cx="23.4" cy="8.6" r="2.7" fill="var(--signal-attention)" />
		</svg>
	)
}

interface WordmarkProps {
	className?: string
}

export function CapitainerieWordmark({ className }: WordmarkProps) {
	return (
		<span
			className={cn(
				'text-[15px] font-bold tracking-[-0.01em] whitespace-nowrap',
				className,
			)}
		>
			Capitainerie
		</span>
	)
}
