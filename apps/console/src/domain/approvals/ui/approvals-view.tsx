import { ShieldCheck } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import type { DemoApproval } from '#/domain/approvals/mock/history'

interface Props {
	history: ReadonlyArray<DemoApproval>
}

export function ApprovalsView({ history }: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Approbations"
				description="Décisions demandées au board"
			/>
			<EmptyState
				icon={ShieldCheck}
				title="Aucune approbation en attente"
				description="Quand un agent demande une validation (dépense, merge, envoi externe), elle apparaît ici."
			/>
			<SectionCard title="Historique récent" flush>
				<ul className="divide-y">
					{history.map((h) => (
						<li key={h.title} className="flex items-center gap-3 px-3 py-2">
							<Badge variant={h.approved ? 'success' : 'danger'}>
								{h.approved ? 'Approuvée' : 'Refusée'}
							</Badge>
							<div className="flex min-w-0 flex-1 flex-col">
								<span className="truncate font-medium">{h.title}</span>
								<span className="text-muted-foreground">{h.decision}</span>
							</div>
							<span className="text-muted-foreground">{h.date}</span>
						</li>
					))}
				</ul>
			</SectionCard>
		</Page>
	)
}
