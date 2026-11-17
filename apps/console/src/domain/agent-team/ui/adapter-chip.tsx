import { ADAPTERS } from '#/domain/agent-team/constants'

interface Props {
	adapter: string
}

export function AdapterChip({ adapter }: Props) {
	const known = ADAPTERS.find((a) => a.name === adapter) ?? ADAPTERS[0]
	return (
		<span
			aria-hidden="true"
			className="inline-flex size-5 shrink-0 items-center justify-center rounded-md text-[10px] font-bold text-white"
			style={{
				backgroundColor: known?.hue
					? `hsl(${known.hue} 55% 42%)`
					: 'var(--muted-foreground)',
			}}
		>
			{known?.initial}
		</span>
	)
}
