import type { CriterionResult } from '#/domain/sources/criteria'
import { Sev } from '#/domain/sources/ui/mockup'

const RESULTS = {
	ok: { label: 'Oui', tone: 'ok' },
	ko: { label: 'Non', tone: 'crit' },
	unknown: { label: 'Incertain', tone: 'warn' },
} as const

export const resultLabel = (result: CriterionResult) => RESULTS[result].label

export function ResultBadge({ result }: { result: CriterionResult }) {
	const { label, tone } = RESULTS[result]
	return <Sev tone={tone}>{label}</Sev>
}
