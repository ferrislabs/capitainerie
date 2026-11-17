import { Plus, X } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import type { EnvVar } from '#/domain/agent-team/agent-config'
import { CTL, Note } from '#/domain/agent-team/ui/field-row'
import { Options } from '#/domain/agent-team/ui/options'
import { ROW, Rows, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'

interface Props {
	env: ReadonlyArray<EnvVar>
	secrets: ReadonlyArray<string>
	onChange: (env: ReadonlyArray<EnvVar>) => void
	onNotify: (message: string) => void
}

const ENV_GRID =
	'grid grid-cols-[minmax(0,1.2fr)_minmax(0,1.2fr)_90px_28px] items-center gap-2 px-3 py-1.5'

export function SecretsTab({ env, secrets, onChange, onNotify }: Props) {
	const patch = (index: number, change: Partial<EnvVar>) =>
		onChange(env.map((row, i) => (i === index ? { ...row, ...change } : row)))
	return (
		<>
			<SectionCard title="Variables d'environnement" flush>
				<div className={`${ENV_GRID} font-mono text-[11.5px] text-faint`}>
					<span>NOM</span>
					<span>SECRET</span>
					<span>VERSION</span>
					<span />
				</div>
				<div>
					{env.map((row, index) => (
						<div key={row.id} className={`${ENV_GRID} border-t`}>
							<input
								aria-label="Nom de la variable"
								className={CTL}
								placeholder="GH_TOKEN"
								value={row.name}
								onChange={(event) => patch(index, { name: event.target.value })}
							/>
							<select
								aria-label="Secret"
								className={CTL}
								value={row.secret}
								onChange={(event) =>
									patch(index, { secret: event.target.value })
								}
							>
								<Options values={secrets} />
							</select>
							<input
								aria-label="Version"
								className={CTL}
								value={row.version}
								onChange={(event) =>
									patch(index, { version: event.target.value })
								}
							/>
							<button
								type="button"
								aria-label="Supprimer la variable"
								onClick={() => onChange(env.filter((_, i) => i !== index))}
								className="grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
							>
								<X aria-hidden="true" className="size-4" />
							</button>
						</div>
					))}
				</div>
				<div className="px-3 py-2">
					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={() =>
							onChange([
								...env,
								{
									id: crypto.randomUUID(),
									name: '',
									secret: secrets[0] ?? '',
									version: 'latest',
								},
							])
						}
					>
						<Plus aria-hidden="true" />
						Ajouter une variable
					</Button>
				</div>
				<Note>
					Donne à NOM le nom de variable attendu par le processus, par exemple
					GH_TOKEN, puis choisis un secret : sa valeur est résolue au démarrage
					du run. Les variables PAPERCLIP_* sont injectées automatiquement.
				</Note>
			</SectionCard>
			<SectionCard
				title="Secrets du coffre"
				flush
				actions={
					<button type="button" onClick={() => onNotify('Nouveau secret')}>
						Nouveau secret
					</button>
				}
			>
				<Rows>
					{secrets.map((secret) => (
						<div key={secret} className={ROW}>
							<Ttl
								title={secret}
								titleMono
								sub={`Valeur masquée · utilisé par ${secret === 'gitlab-com-natalia' ? '4' : '1'} agents`}
							/>
							<Sev tone="ok">latest</Sev>
						</div>
					))}
				</Rows>
			</SectionCard>
		</>
	)
}
