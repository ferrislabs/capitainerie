import type { ReactNode } from 'react'

interface Props {
	title: ReactNode
	/** Texte gris à droite du titre, sur la même ligne. */
	description?: ReactNode
	actions?: ReactNode
	leading?: ReactNode
	meta?: ReactNode
}

/** Titre de page de la maquette : h1 de 20 px, description grise en ligne, actions à droite. */
export function PageHeader({
	title,
	description,
	actions,
	leading,
	meta,
}: Props) {
	return (
		<header className="mb-3 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
			{leading}
			<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
				{title}
			</h1>
			{meta}
			{description ? (
				<span className="min-w-0 text-muted-foreground">{description}</span>
			) : null}
			{actions ? (
				<div className="ml-auto flex flex-wrap items-center gap-2">
					{actions}
				</div>
			) : null}
		</header>
	)
}
