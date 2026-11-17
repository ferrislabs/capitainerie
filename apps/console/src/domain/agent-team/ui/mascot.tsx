import { hueOf } from '#/domain/agent-team/agent-identity'

interface Props {
	name: string
}

/** Pastille qui prend la couleur du nom saisi ; grise tant que le nom est vide. */
export function Mascot({ name }: Props) {
	const awake = name.trim().length > 0
	const background = awake
		? `hsl(${hueOf(name)} 70% 58%)`
		: 'var(--muted-foreground)'
	return (
		<span
			aria-hidden="true"
			className="inline-flex size-16 items-center justify-center rounded-full text-2xl text-white"
			style={{ backgroundColor: background }}
		>
			{awake ? '◡' : 'z'}
		</span>
	)
}
