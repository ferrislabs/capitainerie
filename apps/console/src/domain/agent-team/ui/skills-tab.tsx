import { Badge } from '#/components/ui/badge'
import { Switch } from '#/components/ui/switch'
import { SKILL_ALL } from '#/domain/agent-team/constants'

interface Props {
	skills: ReadonlyArray<string>
	onToggle: (skill: string) => void
}

export function SkillsTab({ skills, onToggle }: Props) {
	return (
		<div className="flex flex-col gap-3">
			<p className="text-muted-foreground">
				<b className="text-foreground">
					{skills.length} sur {SKILL_ALL.length}
				</b>{' '}
				activées · appliqué au prochain run ·{' '}
				<Badge variant="success">Enregistré</Badge>
			</p>
			<ul className="divide-y overflow-hidden rounded-lg border bg-card">
				{SKILL_ALL.map((skill) => (
					<li key={skill} className="flex items-center gap-3 px-3 py-2.5">
						<span className="min-w-0 flex-1">
							<b className="block font-medium">{skill}</b>
							<span className="text-xs text-muted-foreground">
								Compétence fournie par Paperclip.
							</span>
						</span>
						<Badge variant="neutral">Intégré</Badge>
						<Switch
							aria-label={`Activer ${skill}`}
							checked={skills.includes(skill)}
							onCheckedChange={() => onToggle(skill)}
						/>
					</li>
				))}
			</ul>
		</div>
	)
}
