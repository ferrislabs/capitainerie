import { Link } from '@tanstack/react-router'
import { ExternalLink } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { StatusBadge } from '#/components/layout/status-badge'
import { relativeAge } from '#/domain/dashboard/format'
import { Tbl, Td, Th, Tr } from '#/domain/sources/ui/kit'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'
import { worksiteStatus } from '#/domain/worksites/worksite-status'

interface Props {
	items: ReadonlyArray<Schemas.WorksiteResponse>
	now: number
}

export function WorksitesTable({ items, now }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Chantier</Th>
					<Th>Dépôt</Th>
					<Th>Origine</Th>
					<Th>Lancé</Th>
					<Th>État</Th>
				</tr>
			</thead>
			<tbody>
				{items.map((item) => {
					const status = worksiteStatus(item)
					return (
						<Tr key={item.card_id}>
							<Td className="max-w-[420px]">
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(item.card_id) }}
									className="block truncate font-medium text-foreground"
								>
									{item.title}
								</Link>
								<div className="font-mono text-[11.5px] text-muted-foreground">
									{shortTaskId(item.card_id)}
								</div>
							</Td>
							<Td className="font-mono text-[12px] text-muted-foreground">
								{item.repository ?? '—'}
							</Td>
							<Td className="whitespace-nowrap text-muted-foreground">
								{item.issue_url && item.issue_number ? (
									<a
										href={item.issue_url}
										target="_blank"
										rel="noreferrer"
										className="inline-flex items-center gap-1 text-foreground underline underline-offset-2"
									>
										Issue #{item.issue_number}
										<ExternalLink aria-hidden="true" className="size-3" />
									</a>
								) : (
									'Prompt'
								)}
							</Td>
							<Td className="whitespace-nowrap text-muted-foreground">
								<time
									dateTime={item.created_at}
									title={new Date(item.created_at).toLocaleString('fr-FR')}
								>
									{relativeAge(item.created_at, now)}
								</time>
							</Td>
							<Td>
								<StatusBadge tone={status.tone} label={status.label} />
							</Td>
						</Tr>
					)
				})}
			</tbody>
		</Tbl>
	)
}
