import { DEMO_AGENTS, shortName } from '#/domain/demo'
import { cn } from '#/lib/utils'

interface Props {
	name: string
	size?: 'sm' | 'lg'
	className?: string
}

const hashHue = (name: string): number =>
	[...name].reduce((sum, char) => (sum * 31 + char.charCodeAt(0)) % 360, 7)

export function AgentAvatar({ name, size = 'sm', className }: Props) {
	const hue = DEMO_AGENTS.find((a) => a.name === name)?.hue ?? hashHue(name)
	return (
		<span
			aria-hidden="true"
			style={{ backgroundColor: `hsl(${hue} 55% 42%)` }}
			className={cn(
				'inline-flex shrink-0 items-center justify-center rounded-full font-semibold text-white',
				size === 'sm' ? 'size-[18px] text-[10px]' : 'size-9 text-sm',
				className,
			)}
		>
			{shortName(name).charAt(0).toUpperCase()}
		</span>
	)
}

export function AgentLabel({ name }: { name: string }) {
	return (
		<span className="inline-flex items-center gap-1.5">
			<AgentAvatar name={name} />
			{shortName(name)}
		</span>
	)
}
