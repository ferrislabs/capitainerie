import { StatusBadge } from '#/components/layout/status-badge'
import type { RunTone } from '#/domain/routines/types'

interface Props {
	tone: RunTone
	label: string
}

export const RunBadge = ({ tone, label }: Props) => (
	<StatusBadge tone={tone} label={label} />
)
