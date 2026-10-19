import { Bot, CircleDollarSign, ClipboardCheck, Upload } from 'lucide-react'
import { LinkCard } from '#/components/layout/link-card'
import { Badge } from '#/components/ui/badge'
import { Skeleton } from '#/components/ui/skeleton'
import { formatCost } from '#/domain/runs/format'

interface Props {
	toValidate: number | null | undefined
	active: number | null | undefined
	monthCostMicros: number | null | undefined
}

function plural(count: number, one: string, many: string): string {
	return count > 1 ? many : one
}

function Loading() {
	return (
		<>
			<Skeleton className="h-6 w-16" />
			<span className="sr-only">Chargement</span>
		</>
	)
}

function Unavailable() {
	return (
		<span className="text-base font-medium text-muted-foreground">
			Indisponible
		</span>
	)
}

export function HomeCards({ toValidate, active, monthCostMicros }: Props) {
	return (
		<div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
			<LinkCard
				to="/kanban"
				icon={ClipboardCheck}
				title="À valider"
				value={
					toValidate === undefined ? (
						<Loading />
					) : toValidate === null ? (
						<Unavailable />
					) : toValidate > 0 ? (
						<Badge variant="signal" className="px-2 text-base leading-7">
							{toValidate}
						</Badge>
					) : (
						0
					)
				}
				hint={
					typeof toValidate === 'number'
						? toValidate > 0
							? plural(toValidate, 'carte proposée', 'cartes proposées')
							: 'Rien à valider'
						: undefined
				}
			/>
			<LinkCard
				to="/live"
				icon={Bot}
				title="Agents en cours"
				value={
					active === undefined ? (
						<Loading />
					) : active === null ? (
						<Unavailable />
					) : (
						active
					)
				}
				hint={
					typeof active === 'number'
						? plural(active, 'run actif', 'runs actifs')
						: undefined
				}
			/>
			<LinkCard
				to="/agents"
				icon={CircleDollarSign}
				title="Coût du mois"
				value={
					monthCostMicros === undefined ? (
						<Loading />
					) : monthCostMicros === null ? (
						<Unavailable />
					) : (
						formatCost(monthCostMicros)
					)
				}
				hint="Depuis le début du mois"
			/>
			<LinkCard
				to="/runs"
				icon={Upload}
				title="Déposer une réunion"
				hint="Un transcript, un audio ou un PDF."
			/>
		</div>
	)
}
