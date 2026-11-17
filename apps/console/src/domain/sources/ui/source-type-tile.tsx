import type { DemoSourceType } from '#/domain/demo'

interface Props {
	type: DemoSourceType
	size?: number
}

export function SourceTypeTile({ type, size = 28 }: Props) {
	return (
		<span
			aria-hidden="true"
			className="inline-flex shrink-0 items-center justify-center rounded-md bg-muted text-[10px] font-semibold text-muted-foreground"
			style={{ width: size, height: size }}
		>
			{type.slice(0, 3)}
		</span>
	)
}
