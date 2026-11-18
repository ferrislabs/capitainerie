import { InlineAlert } from '#/components/layout/inline-alert'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { formatCount } from '#/domain/dashboard/format'
import { useInbox, useMarkInboxRead } from '#/domain/inbox/hooks/use-inbox'
import { INBOX_TABS, type InboxTab } from '#/domain/inbox/tabs'
import { InboxList } from '#/domain/inbox/ui/inbox-list'
import { Toast } from '#/domain/tasks/ui/toast'

interface Props {
	tab: InboxTab
}

const plural = (count: number, one: string, many: string) =>
	`${formatCount(count)} ${count > 1 ? many : one}`

export function InboxPage({ tab }: Props) {
	const query = useInbox(tab === 'non-lus')
	const markRead = useMarkInboxRead()
	const [message, notify] = useNotice()
	const first = query.data?.pages[0]
	const unread = first?.unread ?? 0
	const items = query.data?.pages.flatMap((page) => page.items) ?? []

	return (
		<Page className="leading-[1.4]">
			<PageHeader
				title="Boîte de réception"
				description={
					first
						? `${plural(first.total, 'élément', 'éléments')} · ${plural(unread, 'non lu', 'non lus')}`
						: undefined
				}
				actions={
					<Button
						variant="outline"
						disabled={unread === 0 || markRead.isPending}
						onClick={() =>
							markRead.mutate(
								{ body: { all: true } },
								{
									onSuccess: (result) =>
										notify(
											`${plural(result.updated, 'élément marqué', 'éléments marqués')} comme lu${result.updated > 1 ? 's' : ''}`,
										),
									onError: () =>
										notify('Impossible de marquer les éléments comme lus'),
								},
							)
						}
					>
						Tout marquer comme lu
					</Button>
				}
			/>
			<TabNav
				label="Filtres de la boîte de réception"
				tabs={INBOX_TABS.map((item) => ({
					label: item.label,
					link: {
						to: '/inbox/{-$tab}',
						params: { tab: item.slug === 'tous' ? undefined : item.slug },
						activeOptions: { exact: true },
					},
				}))}
			/>
			{query.isPending ? (
				<output className="block text-[13px] text-muted-foreground">
					Chargement…
				</output>
			) : query.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger la boîte de réception.
				</InlineAlert>
			) : (
				<>
					<InboxList
						items={items}
						onOpen={(item) => {
							if (item.unread) markRead.mutate({ body: { ids: [item.id] } })
						}}
					/>
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
			)}
			<Toast message={message} />
		</Page>
	)
}
