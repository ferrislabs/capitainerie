import { Link } from '@tanstack/react-router'
import { Button } from '#/components/ui/button'
import { SKILL_ALL } from '#/domain/agent-team/constants'
import { HumanAvatar } from '#/domain/agent-team/ui/human-avatar'
import { Panel } from '#/domain/agent-team/ui/panel'
import { ROW, Rows, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import { Toggle } from '#/domain/agent-team/ui/toggle'
import { DEMO_SKILLS } from '#/domain/skills/mock/skills'

interface Props {
	skills: ReadonlyArray<string>
	onToggle: (skill: string) => void
}

export function SkillsTab({ skills, onToggle }: Props) {
	return (
		<>
			<div className="mb-2 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
				<span className="flex min-w-0 items-center gap-1.5 text-muted-foreground">
					<b className="font-semibold text-foreground">
						{skills.length} sur 86
					</b>{' '}
					activées · appliqué au prochain run · <Sev tone="ok">Enregistré</Sev>
				</span>
				<div className="ml-auto flex gap-2">
					<Button asChild variant="outline">
						<Link to="/skills">Parcourir le catalogue</Link>
					</Button>
				</div>
			</div>
			<Panel>
				<Rows>
					{SKILL_ALL.map((skill) => {
						const entry = DEMO_SKILLS.find((s) => s.name === skill)
						return (
							<div key={skill} className={ROW}>
								<HumanAvatar
									initials={skill[0]?.toUpperCase() ?? ''}
									size={26}
								/>
								<Ttl
									title={skill}
									sub={
										entry?.description ?? 'Compétence fournie par Paperclip.'
									}
								/>
								<span className="hidden md:inline-flex">
									<Sev tone="info">{entry?.scope ?? 'Intégré'}</Sev>
								</span>
								<Toggle
									label={`Activer ${skill}`}
									checked={skills.includes(skill)}
									onChange={() => onToggle(skill)}
								/>
							</div>
						)
					})}
				</Rows>
			</Panel>
		</>
	)
}
