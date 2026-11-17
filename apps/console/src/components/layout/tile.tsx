import type { ReactNode } from 'react'

interface Props {
	label: string
	value: ReactNode
	hint?: ReactNode
}

/** Chiffre clé : libellé, valeur en grand, précision en gris. */
export function Tile({ label, value, hint }: Props) {
	return (
		<div className="flex min-w-0 flex-col gap-1 rounded-lg border bg-card p-3">
			<span className="text-xs text-muted-foreground">{label}</span>
			<span className="figures text-xl leading-tight font-semibold">
				{value}
			</span>
			{hint ? (
				<span className="text-xs text-muted-foreground">{hint}</span>
			) : null}
		</div>
	)
}
