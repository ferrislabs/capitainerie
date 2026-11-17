import { Link } from '@tanstack/react-router'
import { Shield } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import {
	Eyebrow,
	Panel,
	PanelEmpty,
	Row,
	Rows,
	RowTitle,
	When,
} from '#/domain/audit/ui/rows'
import { Sev } from '#/domain/audit/ui/sev'
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
			<Panel>
				{pending.length === 0 ? (
					<PanelEmpty icon={Shield} title="Aucune approbation en attente">
						Quand un agent demande une validation (dépense, merge, envoi
						externe), elle apparaît ici.
					</PanelEmpty>
				) : (
					<Rows>
						{pending.map((a) => {
							const requester = requesterName(a.requestedBy)
							return (
								<Row key={a.id}>
									<RowTitle title={a.title} sub={a.detail}>
										<span className="block truncate text-xs text-faint">
											{requester ? `Demandée par ${requester} · ` : ''}
											{a.at}
											{a.taskId ? (
												<>
													{' · '}
													<Link
														to="/tasks/$taskId"
														params={{ taskId: taskPathId(a.taskId) }}
														className="text-primary hover:underline"
													>
														{a.taskId}
													</Link>
												</>
											) : null}
										</span>
									</RowTitle>
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
								</Row>
							)
						})}
					</Rows>
				)}
			</Panel>
			<Eyebrow>Historique récent</Eyebrow>
			<Panel>
				<Rows>
					{decided.map((a) => (
						<Row key={a.id}>
							<Sev tone={a.status === 'approved' ? 'ok' : 'crit'}>
								{a.status === 'approved' ? 'Approuvée' : 'Refusée'}
							</Sev>
							<RowTitle
								title={a.title}
								sub={`${a.status === 'approved' ? 'Approuvée' : 'Refusée'} par ${a.decidedBy}`}
							>
								{a.taskId ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: taskPathId(a.taskId) }}
										className="text-xs text-primary hover:underline"
									>
										{a.taskId}
									</Link>
								) : null}
							</RowTitle>
							<When>{a.at}</When>
						</Row>
					))}
				</Rows>
			</Panel>
		</Page>
	)
}
