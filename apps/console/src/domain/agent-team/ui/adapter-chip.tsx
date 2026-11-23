import { chipOf } from '#/domain/agent-team/harness'

interface Props {
	adapter: string
}

export function AdapterChip({ adapter }: Props) {
	const known = chipOf(adapter)
	return (
		<span
			aria-hidden="true"
			className="inline-grid size-[26px] shrink-0 place-items-center rounded-[7px] text-[11px] font-bold text-white"
			style={{
				backgroundColor: known ? `hsl(${known.hue} 55% 42%)` : 'var(--faint)',
			}}
		>
			{known?.initial ?? '?'}
		</span>
	)
}
