import { Link } from '@tanstack/react-router'
import { Shield } from 'lucide-react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import {
	APPROVAL_TABS,
	type Approval,
	type ApprovalStatus,
	type Decision,
	type DecisionOutcome,
} from '#/domain/approvals/model'
import { ApprovalCard } from '#/domain/approvals/ui/approval-card'
import { Panel, PanelEmpty } from '#/domain/audit/ui/rows'
import { taskPathId } from '#/domain/tasks/task-ids'
import { cn } from '#/lib/utils'

interface Props {
	status: ApprovalStatus
	approvals: ReadonlyArray<Approval>
	pendingCount: number
	loading: boolean
	error: boolean
	outcome: DecisionOutcome | null
	decideError: string | null
	busyId: string | null
	requesterName: (agentId: string | null | undefined) => string | null
	onStatusChange: (status: ApprovalStatus) => void
	onRetry: () => void
	onDecide: (id: string, decision: Decision, note: string) => void
}

const EMPTY: Record<ApprovalStatus, string> = {
	pending: 'Aucune approbation en attente',
	approved: 'Aucune approbation accordée',
	rejected: 'Aucune approbation refusée',
	expired: 'Aucune approbation expirée',
}

export function ApprovalsView({
	status,
	approvals,
	pendingCount,
	loading,
	error,
	outcome,
	decideError,
	busyId,
	requesterName,
	onStatusChange,
	onRetry,
	onDecide,
}: Props) {
	return (
		<Page>
			<PageHeader
				title="Approbations"
				description="Décisions demandées par les agents avant d'agir"
			/>
			{outcome ? (
				<InlineAlert tone={outcome.tone} className="mb-3">
					{outcome.message}
					{outcome.cardId ? (
						<>
							{' '}
							<Link
								to="/tasks/$taskId"
								params={{ taskId: taskPathId(outcome.cardId) }}
								className="underline underline-offset-2"
							>
								Voir la tâche
							</Link>
						</>
					) : null}
				</InlineAlert>
			) : null}
			{decideError ? (
				<InlineAlert tone="danger" className="mb-3">
					{decideError}
				</InlineAlert>
			) : null}
			<nav
				aria-label="Statut des approbations"
				className="mb-3.5 flex gap-0.5 overflow-x-auto border-b"
			>
				{APPROVAL_TABS.map((tab) => (
					<button
						key={tab.status}
						type="button"
						aria-pressed={tab.status === status}
						onClick={() => onStatusChange(tab.status)}
						className={cn(
							'-mb-px inline-flex items-center gap-1.5 whitespace-nowrap border-b-2 border-transparent px-2.5 py-[7px] font-medium text-muted-foreground hover:text-foreground',
							'aria-pressed:border-primary aria-pressed:text-foreground',
							'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring',
						)}
					>
						{tab.label}
						{tab.status === 'pending' && pendingCount > 0 ? (
							<span className="figures rounded-lg bg-secondary px-1.5 text-[11px] leading-4 text-foreground">
								{pendingCount}
							</span>
						) : null}
					</button>
				))}
			</nav>
			{error ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger les approbations.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={onRetry}>
							Réessayer
						</Button>
					</div>
				</div>
			) : loading ? (
				<output className="m-0 text-muted-foreground">
					Chargement des approbations…
				</output>
			) : approvals.length === 0 ? (
				<Panel>
					<PanelEmpty icon={Shield} title={EMPTY[status]}>
						{status === 'pending'
							? "Quand un agent demande une validation avant d'agir (envoi externe, écriture, dépense), elle apparaît ici."
							: null}
					</PanelEmpty>
				</Panel>
			) : (
				<div className="grid gap-2.5">
					{approvals.map((a) => (
						<ApprovalCard
							key={a.id}
							approval={a}
							requester={requesterName(a.requested_by_agent_id)}
							busy={busyId === a.id}
							onDecide={onDecide}
						/>
					))}
				</div>
			)}
		</Page>
	)
}
