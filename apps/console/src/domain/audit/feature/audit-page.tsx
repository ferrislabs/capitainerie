import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { useAgentHue } from '#/domain/audit/hooks/use-agent-hue'
import {
	useAgentCosts,
	useAuditEvents,
	useAuditRuns,
	useBudgets,
	useSaveBudgets,
} from '#/domain/audit/hooks/use-audit'
import { describeBudgetError, RUN_STATUSES } from '#/domain/audit/model'
import { AUDIT_TABS, type AuditTab } from '#/domain/audit/tabs'
import { ActivityList } from '#/domain/audit/ui/activity-list'
import { BudgetsList } from '#/domain/audit/ui/budgets-list'
import { CostsPanel } from '#/domain/audit/ui/costs-panel'
import { RunsTable } from '#/domain/audit/ui/runs-table'
import { TimelineList } from '#/domain/audit/ui/timeline-list'

interface Props {
	tab: AuditTab
}

function Loading() {
	return (
		<output className="block text-[13px] text-muted-foreground">
			Chargement…
		</output>
	)
}

function Failure({ children }: { children: string }) {
	return <InlineAlert tone="danger">{children}</InlineAlert>
}

function EventsTab({ timeline }: { timeline: boolean }) {
	const query = useAuditEvents()
	const hueOf = useAgentHue()
	if (query.isPending) return <Loading />
	if (query.isError) return <Failure>Impossible de charger le journal.</Failure>
	const events = query.data.pages.flatMap((page) => page.items)
	return (
		<>
			{timeline ? (
				<TimelineList events={events} />
			) : (
				<ActivityList events={events} hueOf={hueOf} />
			)}
			{query.hasNextPage ? (
				<div className="mt-2.5 flex justify-center">
					<Button
						variant="outline"
						disabled={query.isFetchingNextPage}
						onClick={() => void query.fetchNextPage()}
					>
						{query.isFetchingNextPage ? 'Chargement…' : 'Charger plus'}
					</Button>
				</div>
			) : null}
		</>
	)
}

function RunsTab() {
	const [status, setStatus] = useState<Schemas.RunStatusResponse | ''>('')
	const [agentId, setAgentId] = useState('')
	const agents = useTeamAgents().data?.items ?? []
	const query = useAuditRuns({
		status: status || undefined,
		agentId: agentId || undefined,
	})
	const hueOf = useAgentHue()
	return (
		<>
			<div className="mb-2.5 flex flex-wrap gap-2">
				<NativeSelect
					aria-label="Filtrer par agent"
					value={agentId}
					onChange={(e) => setAgentId(e.target.value)}
					className="w-auto"
				>
					<option value="">Tous les agents</option>
					{agents.map((agent) => (
						<option key={agent.id} value={agent.id}>
							{agent.name}
						</option>
					))}
				</NativeSelect>
				<NativeSelect
					aria-label="Filtrer par statut"
					value={status}
					onChange={(e) =>
						setStatus(e.target.value as Schemas.RunStatusResponse | '')
					}
					className="w-auto"
				>
					<option value="">Tous les statuts</option>
					{RUN_STATUSES.map((item) => (
						<option key={item.status} value={item.status}>
							{item.label}
						</option>
					))}
				</NativeSelect>
				{query.data ? (
					<span className="self-center text-xs text-muted-foreground">
						{query.data.total} au total
					</span>
				) : null}
			</div>
			{query.isPending ? (
				<Loading />
			) : query.isError ? (
				<Failure>Impossible de charger les exécutions.</Failure>
			) : (
				<RunsTable runs={query.data.items} hueOf={hueOf} />
			)}
		</>
	)
}

function CostsTab() {
	const costs = useAgentCosts()
	const budgets = useBudgets()
	const agents = useTeamAgents().data?.items
	const hueOf = useAgentHue()
	if (costs.isPending) return <Loading />
	if (costs.isError) return <Failure>Impossible de charger les coûts.</Failure>
	return (
		<CostsPanel
			costs={costs.data}
			budgets={budgets.data}
			hueOf={hueOf}
			agentName={(key) => agents?.find((a) => a.id === key)?.name ?? key}
		/>
	)
}

function BudgetsTab() {
	const budgets = useBudgets()
	const save = useSaveBudgets()
	const agents = useTeamAgents().data?.items ?? []
	if (budgets.isPending) return <Loading />
	if (budgets.isError) {
		const status = (budgets.error as { status?: number } | null)?.status
		return (
			<Failure>
				{status === 403
					? "Tu n'as pas accès aux budgets de l'organisation."
					: 'Impossible de charger les budgets.'}
			</Failure>
		)
	}
	return (
		<BudgetsList
			budgets={budgets.data}
			agents={agents.map((a) => ({ id: a.id, name: a.name }))}
			saving={save.isPending}
			error={save.isError ? describeBudgetError(save.error) : null}
			onSave={(request, done) =>
				save.mutate({ body: request }, { onSuccess: done })
			}
		/>
	)
}

function AuditTabContent({ tab }: Props) {
	switch (tab) {
		case 'activite':
			return <EventsTab timeline={false} />
		case 'runs':
			return <RunsTab />
		case 'couts':
			return <CostsTab />
		case 'budgets':
			return <BudgetsTab />
		case 'chronologie':
			return <EventsTab timeline />
	}
}

export function AuditPage({ tab }: Props) {
	return (
		<Page>
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
