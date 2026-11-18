import { Link } from '@tanstack/react-router'
import { Button } from '#/components/ui/button'
import {
	Sheet,
	SheetBody,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from '#/components/ui/sheet'
import { Eyebrow } from '#/domain/audit/ui/rows'
import { Sev } from '#/domain/audit/ui/sev'
import { type Skill, sourceLabel } from '#/domain/skills/model'
import { shortName } from '#/lib/agent-name'

export interface SkillAgent {
	id: string
	name: string
}

interface Props {
	skill: Skill | null
	agents: ReadonlyArray<SkillAgent>
	busy: boolean
	onClose: () => void
	onEdit: (skill: Skill) => void
	onArchive: (skill: Skill, archived: boolean) => void
}

export function SkillDetailSheet({
	skill,
	agents,
	busy,
	onClose,
	onEdit,
	onArchive,
}: Props) {
	return (
		<Sheet open={skill !== null} onOpenChange={(open) => !open && onClose()}>
			<SheetContent>
				{skill ? (
					<>
						<SheetHeader>
							<SheetTitle>{skill.name}</SheetTitle>
							<SheetDescription>
								<span className="font-mono">{skill.slug}</span>
								{skill.description ? ` · ${skill.description}` : ''}
							</SheetDescription>
							<div className="flex flex-wrap items-center gap-2">
								{skill.archived ? <Sev tone="warn">Archivée</Sev> : null}
								{skill.source === 'manual' ? (
									<Sev tone="info">{sourceLabel(skill.source)}</Sev>
								) : (
									<a
										href={skill.source}
										target="_blank"
										rel="noreferrer"
										className="truncate text-xs text-primary hover:underline"
									>
										{skill.source}
									</a>
								)}
								<span className="ml-auto flex gap-2">
									<Button
										size="sm"
										variant="outline"
										disabled={busy}
										onClick={() => onArchive(skill, !skill.archived)}
									>
										{skill.archived ? 'Restaurer' : 'Archiver'}
									</Button>
									<Button size="sm" onClick={() => onEdit(skill)}>
										Modifier
									</Button>
								</span>
							</div>
						</SheetHeader>
						<SheetBody>
							<section aria-label="Agents qui l'utilisent">
								<Eyebrow>
									{agents.length === 0
										? 'Aucun agent ne l’utilise'
										: `Utilisée par ${agents.length} ${agents.length > 1 ? 'agents' : 'agent'}`}
								</Eyebrow>
								{agents.length > 0 ? (
									<ul className="m-0 flex list-none flex-wrap gap-1.5 p-0">
										{agents.map((a) => (
											<li key={a.id}>
												<Link
													to="/agents/$agentId/{-$tab}"
													params={{ agentId: a.id, tab: 'skills' }}
													className="rounded-md bg-secondary px-2 py-0.5 text-xs hover:text-primary"
												>
													{shortName(a.name)}
												</Link>
											</li>
										))}
									</ul>
								) : null}
							</section>
							<section aria-label="Contenu">
								<Eyebrow>Contenu</Eyebrow>
								{skill.content ? (
									<pre className="m-0 overflow-x-auto rounded-md border bg-card p-3 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap">
										{skill.content}
									</pre>
								) : (
									<p className="m-0 text-muted-foreground">Aucun contenu.</p>
								)}
							</section>
						</SheetBody>
					</>
				) : null}
			</SheetContent>
		</Sheet>
	)
}
