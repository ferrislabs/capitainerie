import { CircleCheck, ExternalLink } from 'lucide-react'
import { type FormEvent, type ReactNode, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
import { RadioGroup, RadioGroupItem } from '#/components/ui/radio-group'
import { Skeleton } from '#/components/ui/skeleton'
import { RepositoriesSection } from '#/domain/integrations/feature/repositories-section'
import {
	describeDeleteAppError,
	describeDisconnectError,
	describeInstallError,
	describeManifestError,
} from '#/domain/integrations/github-errors'
import {
	useCreateGithubManifest,
	useDeleteGithubApp,
	useDisconnectGithub,
	useGithubInstallUrl,
	useGithubIntegration,
} from '#/domain/integrations/hooks/use-github'
import {
	isValidGithubLogin,
	manifestOwner,
	type OwnerKind,
	postManifest,
} from '#/domain/integrations/manifest'
import { ConfirmDialog } from '#/domain/routines/ui/confirm-dialog'
import { cn } from '#/lib/utils'

interface Props {
	/** Message affiché au retour d'une étape réussie sur GitHub. */
	banner?: string | undefined
}

const DATE = new Intl.DateTimeFormat('fr-FR', {
	day: 'numeric',
	month: 'short',
	year: 'numeric',
})

function formatDate(value: string): string {
	const date = new Date(value)
	return Number.isNaN(date.getTime()) ? value : DATE.format(date)
}

function Step({
	index,
	title,
	done,
	disabled = false,
	children,
}: {
	index: number
	title: string
	done: boolean
	disabled?: boolean
	children: ReactNode
}) {
	return (
		<SectionCard
			title={
				<span className="flex items-center gap-2">
					<span
						aria-hidden="true"
						className={cn(
							'grid size-5 place-items-center rounded-full border text-[11px]',
							done && 'border-signal-success bg-signal-success text-white',
						)}
					>
						{done ? '✓' : index}
					</span>
					{`Étape ${index} · ${title}`}
				</span>
			}
			description={done ? 'Terminée' : disabled ? 'En attente' : undefined}
		>
			<div className={cn('grid gap-3', disabled && 'opacity-60')}>
				{children}
			</div>
		</SectionCard>
	)
}

function CreateAppForm() {
	const manifest = useCreateGithubManifest()
	const [kind, setKind] = useState<OwnerKind>('organization')
	const [login, setLogin] = useState('')
	const [touched, setTouched] = useState(false)
	const loginError =
		kind === 'organization' && touched && !isValidGithubLogin(login)
			? "Indique le login de l'organisation GitHub (lettres, chiffres, tirets)."
			: undefined

	const submit = (event: FormEvent) => {
		event.preventDefault()
		setTouched(true)
		if (kind === 'organization' && !isValidGithubLogin(login)) return
		manifest.mutate(
			{ body: { owner: manifestOwner(kind, login) } },
			{ onSuccess: (result) => postManifest(result) },
		)
	}

	return (
		<form onSubmit={submit} className="grid max-w-[520px] gap-3" noValidate>
			<p className="m-0 text-muted-foreground">
				Capitainerie crée sur GitHub une application privée dédiée à ton
				organisation. Elle demande le minimum : lire et écrire le code et les
				pull requests, lire les issues et les membres.
			</p>
			<Field label="Propriétaire de l'application" group>
				<RadioGroup
					value={kind}
					onValueChange={(value) => setKind(value as OwnerKind)}
				>
					<RadioGroupItem
						value="organization"
						label="Une organisation GitHub"
						description="Recommandé : l'application appartient à l'organisation qui possède les dépôts."
					/>
					<RadioGroupItem value="user" label="Mon compte GitHub personnel" />
				</RadioGroup>
			</Field>
			{kind === 'organization' ? (
				<Field
					label="Login de l'organisation GitHub"
					hint="Tel qu'il apparaît dans l'URL : github.com/<login>"
					error={loginError}
					required
				>
					<Input
						value={login}
						placeholder="acme-corp"
						autoComplete="off"
						onChange={(event) => setLogin(event.target.value)}
						onBlur={() => setTouched(true)}
					/>
				</Field>
			) : null}
			{manifest.isError ? (
				<InlineAlert tone="danger">
					{describeManifestError(manifest.error)}
				</InlineAlert>
			) : null}
			<div>
				<Button type="submit" disabled={manifest.isPending}>
					{manifest.isPending
						? 'Redirection vers GitHub…'
						: "Créer l'application GitHub"}
				</Button>
			</div>
		</form>
	)
}

function AppSummary({ app }: { app: Schemas.GithubAppResponse }) {
	return (
		<p className="m-0 flex flex-wrap items-center gap-x-2 gap-y-1">
			<span className="font-mono">{app.slug}</span>
			<span className="text-muted-foreground">
				sur {app.owner_login} · créée le {formatDate(app.created_at)}
			</span>
			<a
				href={app.html_url}
				target="_blank"
				rel="noreferrer"
				className="inline-flex items-center gap-1 text-primary"
			>
				Voir sur GitHub
				<ExternalLink aria-hidden="true" className="size-3.5" />
			</a>
		</p>
	)
}

export function GithubPage({ banner }: Props) {
	const integration = useGithubIntegration()
	const install = useGithubInstallUrl()
	const disconnect = useDisconnectGithub()
	const deleteApp = useDeleteGithubApp()
	const [notice, setNotice] = useNotice()
	const [confirm, setConfirm] = useState<'disconnect' | 'delete' | null>(null)

	const header = (
		<PageHeader
			title="GitHub"
			description="Relie l'organisation à GitHub pour que les agents travaillent sur des dépôts."
		/>
	)

	if (integration.isPending) {
		return (
			<Page>
				{header}
				<output className="grid gap-2.5">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-28 rounded-lg" />
					<Skeleton className="h-28 rounded-lg" />
				</output>
			</Page>
		)
	}

	if (integration.isError) {
		return (
			<Page>
				{header}
				<div className="grid justify-items-start gap-2">
					<InlineAlert tone="danger">
						Impossible de lire l'état de l'intégration GitHub.
					</InlineAlert>
					<Button variant="outline" onClick={() => void integration.refetch()}>
						Réessayer
					</Button>
				</div>
			</Page>
		)
	}

	const app = integration.data.app ?? null
	const installation = integration.data.installation ?? null

	const goInstall = () =>
		install.mutate(undefined, {
			onSuccess: (result) => window.location.assign(result.url),
		})

	const runConfirmed = () => {
		if (confirm === 'disconnect') {
			disconnect.mutate(undefined, {
				onSuccess: () => setNotice('GitHub déconnecté'),
				onError: (error) => setNotice(describeDisconnectError(error)),
			})
		}
		if (confirm === 'delete') {
			deleteApp.mutate(undefined, {
				onSuccess: () =>
					setNotice(
						'Application oubliée par Capitainerie. Supprime-la aussi sur GitHub.',
					),
				onError: (error) => setNotice(describeDeleteAppError(error)),
			})
		}
		setConfirm(null)
	}

	return (
		<Page>
			{header}
			<Notice message={notice} className="mb-2.5" />
			{banner ? (
				<InlineAlert tone="success" className="mb-2.5">
					{banner}
				</InlineAlert>
			) : null}

			{installation && app ? (
				<>
					<SectionCard
						title={
							<span className="flex items-center gap-2">
								<CircleCheck
									aria-hidden="true"
									className="size-4 text-signal-success"
								/>
								GitHub connecté
							</span>
						}
						actions={
							<Button
								size="sm"
								variant="outline"
								disabled={disconnect.isPending}
								onClick={() => setConfirm('disconnect')}
							>
								Déconnecter
							</Button>
						}
					>
						<div className="grid gap-1.5">
							<p className="m-0">
								Installée sur{' '}
								<b className="font-semibold">{installation.account_login}</b>{' '}
								<span className="text-muted-foreground">
									· connectée par {installation.connected_by} le{' '}
									{formatDate(installation.connected_at)}
								</span>
							</p>
							<AppSummary app={app} />
						</div>
					</SectionCard>
					<RepositoriesSection />
				</>
			) : (
				<>
					<Step
						index={1}
						title="Créer l'application GitHub"
						done={app !== null}
					>
						{app ? (
							<div className="grid justify-items-start gap-2">
								<AppSummary app={app} />
								<Button
									size="sm"
									variant="ghost"
									disabled={deleteApp.isPending}
									onClick={() => setConfirm('delete')}
								>
									Oublier cette application
								</Button>
							</div>
						) : (
							<CreateAppForm />
						)}
					</Step>
					<Step
						index={2}
						title="Installer sur GitHub"
						done={false}
						disabled={!app}
					>
						<p className="m-0 text-muted-foreground">
							Choisis sur GitHub les dépôts auxquels l'application a accès. Tu
							reviendras ici automatiquement.
						</p>
						{install.isError ? (
							<InlineAlert tone="danger">
								{describeInstallError(install.error)}
							</InlineAlert>
						) : null}
						<div>
							<Button onClick={goInstall} disabled={!app || install.isPending}>
								{install.isPending
									? 'Redirection vers GitHub…'
									: 'Installer sur GitHub'}
							</Button>
						</div>
					</Step>
				</>
			)}

			<ConfirmDialog
				open={confirm !== null}
				title={
					confirm === 'delete'
						? "Oublier l'application GitHub ?"
						: 'Déconnecter GitHub ?'
				}
				description={
					confirm === 'delete'
						? "Capitainerie oublie l'application et son installation. L'application reste sur GitHub : supprime-la depuis sa page GitHub."
						: "Les agents ne pourront plus travailler sur les dépôts. Les exécutions en cours gardent l'accès qu'elles ont déjà."
				}
				confirmLabel={confirm === 'delete' ? 'Oublier' : 'Déconnecter'}
				onOpenChange={(open) => {
					if (!open) setConfirm(null)
				}}
				onConfirm={runConfirmed}
			/>
		</Page>
	)
}
