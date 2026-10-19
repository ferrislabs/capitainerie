import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useNewConversation } from '#/domain/chat/hooks/use-new-conversation'
import { firstName, greetingTitle } from '#/domain/home/greeting'
import { useMonthCost } from '#/domain/home/hooks/use-month-cost'
import { HomeCards } from '#/domain/home/ui/home-cards'
import { HomeChatSlot } from '#/domain/home/ui/home-chat-slot'
import { RecentRuns } from '#/domain/home/ui/recent-runs'
import { useMe } from '#/domain/identity/hooks/use-me'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { useActiveRuns } from '#/domain/runs/hooks/use-active-runs'
import { useRuns } from '#/domain/runs/hooks/use-runs'

function count<T>(query: {
	isError: boolean
	data: T | undefined
	select: (data: T) => number
}): number | null | undefined {
	if (query.data !== undefined) return query.select(query.data)
	return query.isError ? null : undefined
}

export function HomePage() {
	const me = useMe()
	const cards = useCards()
	const active = useActiveRuns()
	const cost = useMonthCost()
	const runs = useRuns()
	const chat = useNewConversation()

	return (
		<Page>
			<HomeChatSlot
				value={chat.value}
				onChange={chat.setValue}
				onSubmit={chat.submit}
				isPending={chat.isPending}
				errorMessage={chat.errorMessage}
				voice={{
					state: chat.voice.state,
					supported: chat.voice.supported,
					onToggle: chat.voice.toggle,
					onCancel: chat.voice.cancel,
				}}
				voiceError={chat.voice.error}
				model={chat.captain}
			/>
			<PageHeader
				title={greetingTitle(new Date(), firstName(me.data))}
				description="Ce qui demande votre attention et ce que font les agents."
			/>
			<HomeCards
				toValidate={count({
					isError: cards.isError,
					data: cards.data,
					select: (data) =>
						data.items.filter((card) => card.column === 'proposed').length,
				})}
				active={count({
					isError: active.isError,
					data: active.data,
					select: (data) => data.items.length,
				})}
				monthCostMicros={count({
					isError: cost.isError,
					data: cost.data,
					select: (data) => data.total.cost_micros,
				})}
			/>
			<SectionCard title="Activité récente" flush>
				{runs.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger l’activité récente.
					</InlineAlert>
				) : runs.isPending ? (
					<TableSkeleton rows={3} columns={3} />
				) : (
					<RecentRuns runs={runs.data.items} />
				)}
			</SectionCard>
		</Page>
	)
}
