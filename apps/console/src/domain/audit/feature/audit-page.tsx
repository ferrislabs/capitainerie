import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import {
	ACTIVITY,
	AGENT_COSTS,
	AUDIT_RUNS,
	BUDGETS,
	COST_TILES,
	DAILY_RUNS,
	TIMELINE,
} from '#/domain/audit/mock/audit-data'
import { AUDIT_TABS, type AuditTab } from '#/domain/audit/tabs'
import { ActivityList } from '#/domain/audit/ui/activity-list'
import { BudgetsList } from '#/domain/audit/ui/budgets-list'
import { CostsPanel } from '#/domain/audit/ui/costs-panel'
import { RunsTable } from '#/domain/audit/ui/runs-table'
import { TimelineList } from '#/domain/audit/ui/timeline-list'

interface Props {
	tab: AuditTab
}

function AuditTabContent({ tab }: Props) {
	switch (tab) {
		case 'activite':
			return <ActivityList events={ACTIVITY} />
		case 'runs':
			return <RunsTable runs={AUDIT_RUNS} />
		case 'couts':
			return (
				<CostsPanel tiles={COST_TILES} days={DAILY_RUNS} agents={AGENT_COSTS} />
			)
		case 'budgets':
			return <BudgetsList budgets={BUDGETS} />
		case 'chronologie':
			return <TimelineList events={TIMELINE} />
	}
}

export function AuditPage({ tab }: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Audit"
				description="Qui a fait quoi, combien ça a coûté"
			/>
			<TabNav
				label="Sections de l'audit"
				tabs={AUDIT_TABS.map((item) => ({
					label: item.label,
					link: { to: '/audit/{-$tab}', params: { tab: item.slug } },
				}))}
			/>
			<AuditTabContent tab={tab} />
		</Page>
	)
}
