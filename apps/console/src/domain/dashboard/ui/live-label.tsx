import { cn } from '#/lib/utils'

interface Props {
	children: string
	className?: string
}

export function LiveLabel({ children, className }: Props) {
	return (
		<span
			className={cn(
				'inline-flex items-center gap-[5px] text-[11px] whitespace-nowrap text-primary',
				className,
			)}
		>
			<span
				aria-hidden="true"
				className="size-1.5 rounded-full bg-primary motion-safe:animate-[signal-pulse_1.6s_infinite]"
			/>
			{children}
		</span>
	)
}
