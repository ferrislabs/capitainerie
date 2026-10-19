import { Link } from '@tanstack/react-router'
import { Archive } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { relativeDate } from '#/domain/kanban/relative-date'
import { formatTokens } from '#/domain/runs/format'

interface Props {
	conversations: ReadonlyArray<Schemas.ConversationSummaryResponse>
	archivingId?: string
	onArchive: (id: string) => void
}

export function ConversationsTable({
	conversations,
	archivingId,
	onArchive,
}: Props) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Titre</TableHead>
					<TableHead className="hidden md:table-cell">Modèle</TableHead>
					<TableHead className="text-right">Tokens</TableHead>
					<TableHead>Mise à jour</TableHead>
					<TableHead>
						<span className="sr-only">Actions</span>
					</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{conversations.map((conversation) => (
					<TableRow key={conversation.id}>
						<TableCell className="max-w-[28rem]">
							<Link
								to="/chat/$id"
								params={{ id: conversation.id }}
								className="block truncate font-medium text-foreground hover:text-link hover:underline"
							>
								{conversation.title}
							</Link>
						</TableCell>
						<TableCell className="hidden font-mono text-xs text-muted-foreground md:table-cell">
							{conversation.model ?? '—'}
						</TableCell>
						<TableCell className="figures text-right">
							{formatTokens(
								conversation.input_tokens + conversation.output_tokens,
							)}
						</TableCell>
						<TableCell className="text-muted-foreground">
							{relativeDate(conversation.updated_at)}
						</TableCell>
						<TableCell className="text-right">
							<Button
								type="button"
								variant="ghost"
								size="sm"
								aria-label={`Archiver ${conversation.title}`}
								disabled={archivingId === conversation.id}
								onClick={() => onArchive(conversation.id)}
							>
								<Archive aria-hidden="true" />
								Archiver
							</Button>
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
