import { FlaskConical, KeyRound, Trash2 } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { StatusBadge } from '#/components/layout/status-badge'
import { Button } from '#/components/ui/button'
import { formatDateTime } from '#/domain/settings/format'
import type { TestOutcome } from '#/domain/settings/hooks/use-provider-mutations'
import {
	PROVIDER_INFO,
	PROVIDER_KINDS,
	type ProviderKind,
} from '#/domain/settings/providers'

interface Props {
	providers: ReadonlyArray<Schemas.ProviderResponse>
	isAdmin: boolean
	currentSubject?: string
	testResults: Partial<Record<ProviderKind, TestOutcome>>
	testingKind?: ProviderKind
	onConfigure: (kind: ProviderKind) => void
	onTest: (kind: ProviderKind) => void
	onDelete: (kind: ProviderKind) => void
}

export function keyPreview(preview: string): string {
	if (!preview) return 'Sans clé'
	return preview.includes('…') ? preview : `…${preview}`
}

export function configureLabel(
	kind: ProviderKind,
	configured: boolean,
): string {
	if (!configured) return 'Configurer'
	return PROVIDER_INFO[kind].usesBaseUrl ? 'Modifier' : 'Remplacer la clé'
}

function testMessage(outcome: TestOutcome): string {
	if (!outcome.ok) return outcome.error
	const count = outcome.modelCount
	return `Connexion réussie : ${count} modèle${count > 1 ? 's' : ''} annoncé${count > 1 ? 's' : ''}.`
}

export function ProvidersList({
	providers,
	isAdmin,
	currentSubject,
	testResults,
	testingKind,
	onConfigure,
	onTest,
	onDelete,
}: Props) {
	return (
		<ul className="flex flex-col divide-y">
			{PROVIDER_KINDS.map((kind) => {
				const info = PROVIDER_INFO[kind]
				const provider = providers.find((item) => item.kind === kind)
				const configured = provider !== undefined
				const outcome = testResults[kind]
				const testing = testingKind === kind
				const titleId = `provider-${kind}-title`

				return (
					<li
						key={kind}
						aria-labelledby={titleId}
						className="flex flex-col gap-3 px-4 py-4"
					>
						<div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
							<div className="flex min-w-0 flex-col gap-1.5">
								<div className="flex flex-wrap items-center gap-2">
									<h3 id={titleId} className="text-sm font-semibold">
										{info.label}
									</h3>
									<StatusBadge
										tone={configured ? 'success' : 'neutral'}
										label={configured ? 'Configuré' : 'Non configuré'}
									/>
								</div>
								<p className="max-w-[60ch] text-xs text-muted-foreground">
									{info.help}
								</p>
								{provider ? (
									<dl className="mt-0.5 flex flex-wrap gap-x-5 gap-y-1 text-xs">
										<div className="flex items-center gap-1.5">
											<dt className="text-muted-foreground">Clé</dt>
											<dd className="font-mono">
												{keyPreview(provider.preview)}
											</dd>
										</div>
										{info.usesBaseUrl && provider.base_url ? (
											<div className="flex min-w-0 items-center gap-1.5">
												<dt className="text-muted-foreground">URL</dt>
												<dd className="truncate font-mono">
													{provider.base_url}
												</dd>
											</div>
										) : null}
										<div className="flex items-center gap-1.5">
											<dt className="text-muted-foreground">Modifié</dt>
											<dd>
												par{' '}
												{provider.created_by === currentSubject ? (
													'vous'
												) : (
													<span
														title={provider.created_by}
														className="inline-block max-w-[16ch] truncate align-bottom font-mono"
													>
														{provider.created_by}
													</span>
												)}
												, le{' '}
												<span className="figures">
													{formatDateTime(provider.updated_at)}
												</span>
											</dd>
										</div>
									</dl>
								) : null}
							</div>
							{isAdmin ? (
								<div className="flex shrink-0 flex-wrap items-center gap-2">
									{configured ? (
										<Button
											type="button"
											variant="outline"
											size="sm"
											disabled={testing}
											aria-label={`Tester ${info.label}`}
											onClick={() => onTest(kind)}
										>
											<FlaskConical aria-hidden="true" />
											{testing ? 'Test…' : 'Tester'}
										</Button>
									) : null}
									<Button
										type="button"
										variant={configured ? 'outline' : 'default'}
										size="sm"
										aria-label={`${configureLabel(kind, configured)} : ${info.label}`}
										onClick={() => onConfigure(kind)}
									>
										<KeyRound aria-hidden="true" />
										{configureLabel(kind, configured)}
									</Button>
									{configured ? (
										<Button
											type="button"
											variant="ghost"
											size="sm"
											className="text-signal-danger hover:bg-signal-danger-soft hover:text-signal-danger"
											aria-label={`Supprimer ${info.label}`}
											onClick={() => onDelete(kind)}
										>
											<Trash2 aria-hidden="true" />
											Supprimer
										</Button>
									) : null}
								</div>
							) : null}
						</div>
						{outcome ? (
							<InlineAlert tone={outcome.ok ? 'success' : 'danger'}>
								{testMessage(outcome)}
							</InlineAlert>
						) : null}
					</li>
				)
			})}
		</ul>
	)
}
