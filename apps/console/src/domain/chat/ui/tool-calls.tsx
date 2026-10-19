import { ChevronRight, Loader2 } from 'lucide-react'
import type { ToolView } from '#/domain/chat/chat-state'
import { toolHeadline } from '#/domain/chat/tool-label'
import { formatLatency } from '#/domain/runs/format'
import { cn } from '#/lib/utils'

interface Props {
	tools: ReadonlyArray<ToolView>
}

function ToolRow({ tool }: { tool: ToolView }) {
	return (
		<li>
			<details className="group">
				<summary
					className={cn(
						'flex cursor-pointer list-none items-center gap-1.5 rounded-sm py-0.5 text-[13px] text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden',
						tool.status === 'failed' && 'text-signal-danger',
					)}
				>
					{tool.status === 'running' ? (
						<Loader2 aria-hidden="true" className="size-3.5 animate-spin" />
					) : (
						<ChevronRight
							aria-hidden="true"
							className="size-3.5 transition-transform duration-150 group-open:rotate-90"
						/>
					)}
					{toolHeadline(tool)}
				</summary>
				<dl className="mt-1 mb-1.5 ml-5 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-0.5 text-xs">
					<dt className="text-muted-foreground">Outil</dt>
					<dd className="font-mono">{tool.name}</dd>
					{tool.arguments ? (
						<>
							<dt className="text-muted-foreground">Arguments</dt>
							<dd className="font-mono break-all">{tool.arguments}</dd>
						</>
					) : null}
					{tool.summary ? (
						<>
							<dt className="text-muted-foreground">Résultat</dt>
							<dd>{tool.summary}</dd>
						</>
					) : null}
					{tool.latencyMs !== undefined ? (
						<>
							<dt className="text-muted-foreground">Durée</dt>
							<dd className="figures font-mono">
								{formatLatency(tool.latencyMs)}
							</dd>
						</>
					) : null}
				</dl>
			</details>
		</li>
	)
}

export function ToolCalls({ tools }: Props) {
	if (tools.length === 0) return null
	return (
		<ul aria-label="Appels d’outils" className="flex flex-col">
			{tools.map((tool) => (
				<ToolRow key={tool.id} tool={tool} />
			))}
		</ul>
	)
}
