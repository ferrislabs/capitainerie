import type { RunTone } from '#/domain/routines/types'
import { Sev, type SevTone } from '#/domain/sources/ui/mockup'

interface Props {
	tone: RunTone
	label: string
	listing?: boolean
}

const TONES: Record<RunTone, SevTone> = {
	success: 'ok',
	danger: 'crit',
	neutral: 'info',
	violet: 'info',
}

export const RunBadge = ({ tone, label, listing = false }: Props) => (
	<Sev tone={listing && tone === 'violet' ? 'vio' : TONES[tone]}>{label}</Sev>
)
