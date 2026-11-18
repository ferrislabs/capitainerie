import { useNavigate } from '@tanstack/react-router'
import { Loader2 } from 'lucide-react'
import { type ReactNode, useEffect, useRef, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import {
	describeConnectError,
	describeCreateAppError,
} from '#/domain/integrations/github-errors'
import {
	useConnectGithub,
	useCreateGithubApp,
} from '#/domain/integrations/hooks/use-github'
import { hasErrorKey } from '#/lib/api-error'

export type GithubReturn = 'app' | 'installation'

function Shell({
	title,
	problem,
	info,
	pending,
}: {
	title: string
	problem: string | null
	info?: ReactNode
	pending: string
}) {
	const navigate = useNavigate()
	const back = (
		<div>
			<Button
				variant="outline"
				onClick={() => void navigate({ to: '/settings/integrations/github' })}
			>
				Retour à GitHub
			</Button>
		</div>
	)
	return (
		<Page>
			<PageHeader title={title} />
			{problem ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">{problem}</InlineAlert>
					{back}
				</div>
			) : info ? (
				<div className="grid gap-2">
					<InlineAlert tone="info">{info}</InlineAlert>
					{back}
				</div>
			) : (
				<output className="m-0 flex items-center gap-2 text-muted-foreground">
					<Loader2 aria-hidden="true" className="size-4 animate-spin" />
					{pending}
				</output>
			)}
		</Page>
	)
}

function useLeaveTo(): (step: GithubReturn) => void {
	const navigate = useNavigate()
	return (step) =>
		void navigate({
			to: '/settings/integrations/github',
			search: { retour: step },
			replace: true,
		})
}

interface AppCallbackProps {
	code: string | undefined
	state: string | undefined
	error?: string | undefined
}

/** Retour de GitHub après la création de l'application par manifeste. */
export function GithubAppCallbackPage({
	code,
	state,
	error,
}: AppCallbackProps) {
	const { mutateAsync } = useCreateGithubApp()
	const leave = useLeaveTo()
	const submitted = useRef(false)
	const [failure, setFailure] = useState<unknown>(null)

	const missing = error
		? `GitHub n'a pas créé l'application (${error}).`
		: !code || !state
			? "Retour de GitHub incomplet : le code ou l'état est manquant. Recommence la création."
			: null

	useEffect(() => {
		if (missing || !code || !state || submitted.current) return
		submitted.current = true
		mutateAsync({ body: { code, state } }).then(
			() => leave('app'),
			(error: unknown) =>
				hasErrorKey(error, 'github_app_exists')
					? leave('app')
					: setFailure(error ?? new Error('échec')),
		)
	}, [missing, code, state, mutateAsync, leave])

	return (
		<Shell
			title="Création de l'application GitHub"
			problem={missing ?? (failure ? describeCreateAppError(failure) : null)}
			pending="Enregistrement de l'application…"
		/>
	)
}

interface InstallCallbackProps {
	installationId: number | undefined
	setupAction: string | undefined
	code: string | undefined
	state: string | undefined
}

/** Retour de GitHub après l'installation de l'application. */
export function GithubInstallCallbackPage({
	installationId,
	setupAction,
	code,
	state,
}: InstallCallbackProps) {
	const { mutateAsync } = useConnectGithub()
	const leave = useLeaveTo()
	const submitted = useRef(false)
	const [failure, setFailure] = useState<unknown>(null)

	const requested = setupAction === 'request'
	const updatedFromGithub =
		setupAction === 'update' && (!state || installationId === undefined)
	const ready =
		!requested &&
		!updatedFromGithub &&
		installationId !== undefined &&
		Boolean(code) &&
		Boolean(state)

	const missing =
		!requested && !updatedFromGithub && !ready
			? "Retour de GitHub incomplet : l'installation, le code ou l'état est manquant. Relance l'installation depuis la page GitHub."
			: null

	useEffect(() => {
		if (!ready || !code || !state || installationId === undefined) return
		if (submitted.current) return
		submitted.current = true
		mutateAsync({
			body: { installation_id: installationId, code, state },
		}).then(
			() => leave('installation'),
			(error: unknown) => setFailure(error ?? new Error('échec')),
		)
	}, [ready, code, state, installationId, mutateAsync, leave])

	return (
		<Shell
			title="Installation de l'application GitHub"
			problem={missing ?? (failure ? describeConnectError(failure) : null)}
			info={
				requested
					? "Ta demande d'installation a été envoyée aux propriétaires de l'organisation GitHub. Reviens ici une fois qu'ils l'ont acceptée, puis relance l'installation."
					: updatedFromGithub
						? "L'installation a été modifiée sur GitHub. La liste des dépôts accessibles est à jour."
						: undefined
			}
			pending="Connexion de l'installation…"
		/>
	)
}
