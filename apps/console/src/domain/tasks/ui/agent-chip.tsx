interface Props {
	name: string
	hue: number
	size?: number
	showName?: boolean
}

export function AgentChip({ name, hue, size = 18, showName = true }: Props) {
	return (
		<span className="inline-flex items-center gap-1.5">
			<span
				role="img"
				aria-label={name}
				title={name}
				className="inline-flex shrink-0 items-center justify-center rounded-full text-[10px] font-semibold text-white"
				style={{
					width: size,
					height: size,
					background: `oklch(0.55 0.12 ${hue})`,
				}}
			>
				{name.slice(0, 1).toUpperCase()}
			</span>
			{showName ? <span className="text-[12px]">{name}</span> : null}
		</span>
	)
}
