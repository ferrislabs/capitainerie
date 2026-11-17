import { Badge } from '#/components/ui/badge'
import type { CriterionResult, Verdict } from '#/domain/sources/criteria'

const RESULTS = {
	ok: { label: 'Oui', variant: 'success' },
	ko: { label: 'Non', variant: 'danger' },
	unknown: { label: 'Incertain', variant: 'attention' },
} as const

const VERDICTS = {
	qualified: { label: 'Qualifié', variant: 'success' },
	review: { label: 'À valider', variant: 'attention' },
	rejected: { label: 'Rejeté', variant: 'danger' },
} as const

export const resultLabel = (result: CriterionResult) => RESULTS[result].label

export function ResultBadge({ result }: { result: CriterionResult }) {
	const { label, variant } = RESULTS[result]
	return <Badge variant={variant}>{label}</Badge>
}

export function VerdictBadge({ verdict }: { verdict: Verdict }) {
	const { label, variant } = VERDICTS[verdict]
	return <Badge variant={variant}>{label}</Badge>
}
