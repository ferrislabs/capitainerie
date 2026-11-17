import {
	Ban,
	Circle,
	CircleCheck,
	CircleDashed,
	CircleDot,
	CircleSlash,
	LoaderCircle,
} from 'lucide-react'
import type { DemoTaskStatus } from '#/domain/demo'
import { cn } from '#/lib/utils'

const ICONS = {
	todo: { Icon: Circle, tone: 'text-signal-attention-ink' },
	progress: { Icon: LoaderCircle, tone: 'text-signal-running' },
	review: { Icon: CircleDot, tone: 'text-signal-running' },
	done: { Icon: CircleCheck, tone: 'text-signal-success' },
	blocked: { Icon: Ban, tone: 'text-signal-danger' },
	cancelled: { Icon: CircleSlash, tone: 'text-signal-neutral' },
	backlog: { Icon: CircleDashed, tone: 'text-signal-neutral' },
} as const

interface Props {
	status: DemoTaskStatus
	className?: string
}

export function StatusIcon({ status, className }: Props) {
	const { Icon, tone } = ICONS[status]
	return (
		<Icon
			aria-hidden="true"
			className={cn('size-4 shrink-0', tone, className)}
		/>
	)
}
