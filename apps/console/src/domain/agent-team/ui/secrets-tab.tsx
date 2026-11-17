import { Plus, X } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { TextInput } from '#/components/ui/text-input'
import type { EnvVar } from '#/domain/agent-team/agent-config'
import { Options } from '#/domain/agent-team/ui/options'

interface Props {
	env: ReadonlyArray<EnvVar>
	secrets: ReadonlyArray<string>
	onChange: (env: ReadonlyArray<EnvVar>) => void
}

export function SecretsTab({ env, secrets, onChange }: Props) {
	const patch = (index: number, change: Partial<EnvVar>) =>
		onChange(env.map((row, i) => (i === index ? { ...row, ...change } : row)))
	return (
		<div className="flex flex-col gap-4">
			<SectionCard title="Variables d'environnement">
				<div className="flex flex-col gap-2">
					{env.map((row, index) => (
						<div
							key={row.id}
							className="grid grid-cols-[1fr_1fr_6rem_auto] gap-2"
						>
							<TextInput
								aria-label="Nom de la variable"
								placeholder="GH_TOKEN"
								value={row.name}
								onChange={(event) => patch(index, { name: event.target.value })}
							/>
							<NativeSelect
								aria-label="Secret"
								value={row.secret}
								onChange={(event) =>
									patch(index, { secret: event.target.value })
								}
							>
								<Options values={secrets} />
							</NativeSelect>
							<TextInput
								aria-label="Version"
								value={row.version}
								onChange={(event) =>
									patch(index, { version: event.target.value })
								}
							/>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								aria-label="Supprimer la variable"
								onClick={() => onChange(env.filter((_, i) => i !== index))}
							>
								<X aria-hidden="true" />
							</Button>
						</div>
					))}
					<div>
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
					<p className="text-muted-foreground">
						Donne à NOM le nom de variable attendu par le processus, par exemple
						GH_TOKEN, puis choisis un secret : sa valeur est résolue au
						démarrage du run. Les variables PAPERCLIP_* sont injectées
						automatiquement.
					</p>
				</div>
			</SectionCard>
			<SectionCard title="Secrets du coffre" flush>
				<ul className="divide-y">
					{secrets.map((secret) => (
						<li key={secret} className="flex items-center gap-3 px-3 py-2.5">
							<span className="min-w-0 flex-1">
								<b className="block font-mono font-medium">{secret}</b>
								<span className="text-xs text-muted-foreground">
									Valeur masquée · utilisé par{' '}
									{secret === 'gitlab-com-natalia' ? '4' : '1'} agents
								</span>
							</span>
							<Badge variant="success">latest</Badge>
						</li>
					))}
				</ul>
			</SectionCard>
		</div>
	)
}
