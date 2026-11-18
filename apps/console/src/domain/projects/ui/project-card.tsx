import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'

interface Props {
	item: Schemas.ProjectListItemResponse
	progress: number | null
	busy: boolean
	onEdit: () => void
	onToggleArchive: () => void
}

export function ProjectCard({
	item,
	progress,
	busy,
	onEdit,
	onToggleArchive,
}: Props) {
	const { project, open_cards } = item
	return (
		<article className="flex min-w-0 flex-col gap-2 rounded-lg border bg-card px-3 py-[11px]">
			<h3 className="m-0 text-[13px] font-semibold">{project.name}</h3>
			{project.description ? (
				<p className="m-0 text-muted-foreground">{project.description}</p>
			) : null}
			{project.client || project.repository ? (
				<p className="m-0 flex min-w-0 flex-wrap gap-x-2 font-mono text-[11.5px] text-faint">
					{project.client ? <span>{project.client.name}</span> : null}
					{project.repository ? <span>{project.repository}</span> : null}
				</p>
			) : null}
			{progress === null ? null : (
				<div
					role="progressbar"
					aria-label={`Avancement de ${project.name}`}
					aria-valuenow={progress}
					aria-valuemin={0}
					aria-valuemax={100}
					className="h-1.5 min-w-[60px] overflow-hidden rounded-[3px] bg-secondary"
				>
					<div
						className="h-full rounded-[3px] bg-primary"
						style={{ width: `${progress}%` }}
					/>
				</div>
			)}
			<div className="flex items-center justify-between gap-2">
				<span className="flex gap-1">
					<Button variant="outline" size="sm" onClick={onEdit} disabled={busy}>
						Modifier
					</Button>
					<Button
						variant="ghost"
						size="sm"
						onClick={onToggleArchive}
						disabled={busy}
					>
						{project.archived ? 'Restaurer' : 'Archiver'}
					</Button>
				</span>
				<span className="text-muted-foreground tabular-nums">
					{progress === null ? '' : `${progress} % · `}
					{open_cards} {open_cards > 1 ? 'ouvertes' : 'ouverte'}
				</span>
			</div>
		</article>
	)
}
