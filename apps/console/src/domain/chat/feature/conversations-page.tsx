import { Link } from '@tanstack/react-router'
import { MessageSquare } from 'lucide-react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { Button } from '#/components/ui/button'
import { useArchiveConversation } from '#/domain/chat/hooks/use-archive-conversation'
import { useConversations } from '#/domain/chat/hooks/use-conversations'
import { ConversationsTable } from '#/domain/chat/ui/conversations-table'

export function ConversationsPage() {
	const conversations = useConversations()
	const archive = useArchiveConversation()

	return (
		<Page>
			<PageHeader
				title="Conversations"
				description="Vos échanges avec le Capitaine, les plus récents d’abord."
				actions={
					<Button asChild variant="outline">
						<Link to="/">Nouvelle conversation</Link>
					</Button>
				}
			/>
			<SectionCard title="Récentes" bare>
				{conversations.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger les conversations.
					</InlineAlert>
				) : conversations.isPending ? (
					<TableSkeleton rows={4} columns={4} />
				) : conversations.data.conversations.length === 0 ? (
					<EmptyState
						icon={MessageSquare}
						title="Aucune conversation"
						description="Décrivez une mission depuis l’accueil : le Capitaine propose des cartes que vous validez."
						action={
							<Button asChild variant="outline" size="sm">
								<Link to="/">Lancer une mission</Link>
							</Button>
						}
					/>
				) : (
					<ConversationsTable
						conversations={conversations.data.conversations}
						archivingId={
							archive.isPending ? archive.variables?.path.id : undefined
						}
						onArchive={(id) =>
							archive.mutate({ path: { id }, body: { archived: true } })
						}
					/>
				)}
				{archive.isError ? (
					<InlineAlert tone="danger" className="mt-3">
						Impossible d’archiver cette conversation.
					</InlineAlert>
				) : null}
			</SectionCard>
		</Page>
	)
}
