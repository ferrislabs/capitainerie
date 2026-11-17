import type { DemoSourceType } from '#/domain/demo'

interface Props {
	type: DemoSourceType
	size?: number
}

export function SourceTypeTile({ type, size = 28 }: Props) {
	return (
		<span
			aria-hidden="true"
			className="inline-grid shrink-0 place-items-center rounded-full bg-secondary text-[9px] font-semibold text-muted-foreground"
			style={{ width: size, height: size }}
		>
			{type.slice(0, 3)}
		</span>
	)
}
