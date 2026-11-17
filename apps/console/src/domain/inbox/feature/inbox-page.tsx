import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { filterInbox } from '#/domain/inbox/filter-inbox'
import { INBOX_ITEMS } from '#/domain/inbox/mock/inbox-items'
import { INBOX_TABS, type InboxTab } from '#/domain/inbox/tabs'
import { InboxList } from '#/domain/inbox/ui/inbox-list'
import { Toast } from '#/domain/tasks/ui/toast'

interface Props {
	tab: InboxTab
}

export function InboxPage({ tab }: Props) {
	const [allRead, setAllRead] = useState(false)
	const [message, notify] = useNotice()
	const items = allRead
		? INBOX_ITEMS.map((item) => ({ ...item, unread: false }))
		: INBOX_ITEMS
	const unread = items.filter((item) => item.unread).length
	return (
		<Page className="leading-[1.4]">
			<DemoBanner />
			<PageHeader
				title="Boîte de réception"
				description={`67 éléments · ${unread} non lus ici`}
				actions={
					<Button
						variant="outline"
						disabled={unread === 0}
						onClick={() => {
							setAllRead(true)
							notify('67 éléments marqués comme lus')
						}}
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
			<InboxList items={filterInbox(tab, items)} />
			<Toast message={message} />
		</Page>
	)
}
