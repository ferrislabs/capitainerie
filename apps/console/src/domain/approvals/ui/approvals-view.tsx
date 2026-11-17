import { Link } from '@tanstack/react-router'
import { ShieldCheck } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import type { DemoApproval } from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	pending: ReadonlyArray<DemoApproval>
	decided: ReadonlyArray<DemoApproval>
	requesterName: (agentId: string | null) => string | null
	notice: string | null
	onDecide: (id: string, status: 'approved' | 'rejected') => void
}

export function ApprovalsView({
	pending,
	decided,
	requesterName,
	notice,
	onDecide,
}: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Approbations"
				description="Décisions demandées au board"
			/>
			<Notice message={notice} />
			<SectionCard title="En attente" flush={pending.length > 0}>
				{pending.length === 0 ? (
					<EmptyState
						icon={ShieldCheck}
						title="Aucune approbation en attente"
						description="Quand un agent demande une validation (dépense, merge, envoi externe), elle apparaît ici."
					/>
				) : (
					<ul className="divide-y">
						{pending.map((a) => {
							const requester = requesterName(a.requestedBy)
							return (
								<li key={a.id} className="flex items-center gap-3 px-3 py-2">
									<div className="flex min-w-0 flex-1 flex-col">
										<span className="font-medium">{a.title}</span>
										<span className="text-muted-foreground">{a.detail}</span>
										<span className="text-xs text-muted-foreground">
											{requester ? `Demandée par ${requester} · ` : ''}
											{a.at}
										</span>
										{a.taskId ? (
											<Link
												to="/tasks/$taskId"
												params={{ taskId: taskPathId(a.taskId) }}
												className="text-xs text-link hover:underline"
											>
												{a.taskId}
											</Link>
										) : null}
									</div>
									<Button
										type="button"
										size="sm"
										aria-label={`Approuver : ${a.title}`}
										onClick={() => onDecide(a.id, 'approved')}
									>
										Approuver
									</Button>
									<Button
										type="button"
										size="sm"
										variant="outline"
										aria-label={`Refuser : ${a.title}`}
										onClick={() => onDecide(a.id, 'rejected')}
									>
										Refuser
									</Button>
								</li>
							)
						})}
					</ul>
				)}
			</SectionCard>
			<SectionCard title="Historique récent" flush>
				<ul className="divide-y">
					{decided.map((a) => (
						<li key={a.id} className="flex items-center gap-3 px-3 py-2">
							<Badge variant={a.status === 'approved' ? 'success' : 'danger'}>
								{a.status === 'approved' ? 'Approuvée' : 'Refusée'}
							</Badge>
							<div className="flex min-w-0 flex-1 flex-col">
								<span className="truncate font-medium">{a.title}</span>
								<span className="text-muted-foreground">
									{a.status === 'approved' ? 'Approuvée' : 'Refusée'} par{' '}
									{a.decidedBy}
								</span>
								{a.taskId ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: taskPathId(a.taskId) }}
										className="text-xs text-link hover:underline"
									>
										{a.taskId}
									</Link>
								) : null}
							</div>
							<span className="text-muted-foreground">{a.at}</span>
						</li>
					))}
				</ul>
			</SectionCard>
		</Page>
	)
}
