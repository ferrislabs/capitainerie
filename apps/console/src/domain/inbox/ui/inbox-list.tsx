import { Link, type LinkProps } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { relativeAge } from '#/domain/dashboard/format'
import { type InboxItem, KIND_LABEL, targetOf } from '#/domain/inbox/model'
import { cn } from '#/lib/utils'

interface Props {
	items: ReadonlyArray<InboxItem>
	onOpen: (item: InboxItem) => void
}

const ROW =
	'flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 text-left first:border-t-0 hover:bg-secondary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring'

function linkOf(item: InboxItem): LinkProps | { href: string } {
	const target = targetOf(item)
	switch (target.kind) {
		case 'task':
			return { to: '/tasks/$taskId', params: { taskId: target.taskId } }
		case 'internal':
			return { to: target.href as LinkProps['to'] }
		case 'external':
			return { href: target.href }
		case 'page':
			return target.to === '/tasks'
				? { to: '/tasks', search: { vue: 'liste', q: '' } }
				: target.to === '/audit/{-$tab}'
					? { to: '/audit/{-$tab}', params: { tab: 'runs' } }
					: { to: target.to }
	}
}

function Row({ item, onOpen }: { item: InboxItem; onOpen: Props['onOpen'] }) {
	const content: ReactNode = (
		<>
			<span
				role="img"
				aria-label={item.unread ? 'Non lu' : 'Lu'}
				className={cn(
					'w-[7px] flex-none',
					item.unread && 'h-[7px] rounded-full bg-primary',
				)}
			/>
			<span className="w-[86px] flex-none font-mono text-[11px] font-medium whitespace-nowrap text-faint">
				{KIND_LABEL[item.kind]}
			</span>
			<span className="min-w-0 flex-1">
				<b
					className={cn(
						'block truncate leading-[19px]',
						item.unread ? 'font-semibold' : 'font-medium',
					)}
				>
					{item.title}
				</b>
				{item.body ? (
					<span className="block truncate text-xs leading-[1.4] text-muted-foreground">
						{item.body}
					</span>
				) : null}
			</span>
			<time
				dateTime={item.created_at}
				className="font-mono text-[11.5px] whitespace-nowrap text-faint"
			>
				{relativeAge(item.created_at)}
			</time>
		</>
	)
	const link = linkOf(item)
	if ('href' in link) {
		return (
			<a
				href={link.href}
				target="_blank"
				rel="noopener noreferrer"
				className={ROW}
				onClick={() => onOpen(item)}
			>
				{content}
			</a>
		)
	}
	return (
		<Link {...link} className={ROW} onClick={() => onOpen(item)}>
			{content}
		</Link>
	)
}

export function InboxList({ items, onOpen }: Props) {
	return (
		<section className="overflow-hidden rounded-lg border bg-card">
			{items.length === 0 ? (
				<div className="px-4 py-[26px] text-center text-muted-foreground">
					<b className="mt-1.5 mb-0.5 block font-semibold text-foreground">
						Rien ici
					</b>
					Aucun élément pour ce filtre.
				</div>
			) : (
				<ul className="m-0 list-none p-0">
					{items.map((item) => (
						<li key={item.id}>
							<Row item={item} onOpen={onOpen} />
						</li>
					))}
				</ul>
			)}
		</section>
	)
}
