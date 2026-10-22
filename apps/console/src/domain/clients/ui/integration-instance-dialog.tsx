import { type FormEvent, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import { Input } from '#/components/ui/input'
import { Label } from '#/components/ui/label'
import {
	EMPTY_VALUES,
	type IntegrationValues,
	requiresNewSecret,
	usesDatabase,
	usesLogin,
	validateIntegration,
	valuesOf,
} from '#/domain/clients/integration-form'
import {
	ENVIRONMENT_LABELS,
	ENVIRONMENTS,
	KIND_LABELS,
	KINDS,
} from '#/domain/clients/integration-labels'

const SELECT_CLASS =
	'h-8 w-full rounded-md border border-input bg-card px-3 text-sm text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:opacity-60'

const NEW_KEY_HELP =
	'Une nouvelle clé est obligatoire quand l’adresse, la base ou l’identifiant change.'

const ODOO_HELP =
	'L’API externe d’Odoo Online demande une offre Custom. Pour la production, utilisez de préférence la clé d’un utilisateur Odoo en lecture seule.'

interface Props {
	open: boolean
	instance?: Schemas.IntegrationInstanceResponse
	isPending: boolean
	errorMessage?: string
	onSubmit: (values: IntegrationValues) => void
	onClose: () => void
}

function FieldError({ message }: { message?: string }) {
	return message ? (
		<span role="alert" className="text-xs text-signal-danger">
			{message}
		</span>
	) : null
}

function IntegrationForm({
	instance,
	isPending,
	errorMessage,
	onSubmit,
	onClose,
}: Omit<Props, 'open'>) {
	const mode = instance ? 'edit' : 'create'
	const [values, setValues] = useState<IntegrationValues>(
		instance ? valuesOf(instance) : EMPTY_VALUES,
	)
	const [attempted, setAttempted] = useState(false)
	const errors = validateIntegration(values, mode, instance)
	const secretRequired =
		mode === 'create' ||
		(instance !== undefined && requiresNewSecret(values, instance))
	const invalid = Object.keys(errors).length > 0
	const shown = attempted ? errors : {}

	function set<Key extends keyof IntegrationValues>(
		key: Key,
		value: IntegrationValues[Key],
	) {
		setValues((current) => ({ ...current, [key]: value }))
	}

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setAttempted(true)
		if (invalid) return
		onSubmit(values)
		set('secret', '')
	}

	return (
		<form onSubmit={handleSubmit} className="flex flex-col gap-4" noValidate>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="integration-kind">Type</Label>
				<select
					id="integration-kind"
					className={SELECT_CLASS}
					value={values.kind}
					onChange={(event) =>
						set('kind', event.target.value as Schemas.IntegrationKindDto)
					}
					disabled={isPending || instance !== undefined}
				>
					{KINDS.map((kind) => (
						<option key={kind} value={kind}>
							{KIND_LABELS[kind]}
						</option>
					))}
				</select>
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="integration-name">Nom</Label>
				<Input
					id="integration-name"
					value={values.name}
					onChange={(event) => set('name', event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(shown.name)}
				/>
				<FieldError message={shown.name} />
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="integration-environment">Environnement</Label>
				<select
					id="integration-environment"
					className={SELECT_CLASS}
					value={values.environment}
					onChange={(event) =>
						set(
							'environment',
							event.target.value as Schemas.IntegrationEnvironmentDto,
						)
					}
					disabled={isPending}
				>
					{ENVIRONMENTS.map((environment) => (
						<option key={environment} value={environment}>
							{ENVIRONMENT_LABELS[environment]}
						</option>
					))}
				</select>
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="integration-url">URL</Label>
				<Input
					id="integration-url"
					className="font-mono text-xs"
					value={values.baseUrl}
					onChange={(event) => set('baseUrl', event.target.value)}
					disabled={isPending}
					placeholder="https://"
					aria-invalid={Boolean(shown.baseUrl)}
				/>
				<FieldError message={shown.baseUrl} />
			</div>
			{usesDatabase(values.kind) ? (
				<div className="flex flex-col gap-1.5">
					<Label htmlFor="integration-database">
						{usesLogin(values.kind) ? 'Base' : 'Base (facultative)'}
					</Label>
					<Input
						id="integration-database"
						value={values.database}
						onChange={(event) => set('database', event.target.value)}
						disabled={isPending}
						aria-invalid={Boolean(shown.database)}
					/>
					<FieldError message={shown.database} />
				</div>
			) : null}
			{usesLogin(values.kind) ? (
				<div className="flex flex-col gap-1.5">
					<Label htmlFor="integration-login">Identifiant</Label>
					<Input
						id="integration-login"
						value={values.login}
						onChange={(event) => set('login', event.target.value)}
						disabled={isPending}
						aria-invalid={Boolean(shown.login)}
					/>
					<FieldError message={shown.login} />
				</div>
			) : null}
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="integration-secret">Clé API / mot de passe</Label>
				<Input
					id="integration-secret"
					type="password"
					autoComplete="new-password"
					value={values.secret}
					onChange={(event) => set('secret', event.target.value)}
					disabled={isPending}
					aria-required={secretRequired}
					aria-invalid={Boolean(shown.secret)}
				/>
				<FieldError message={shown.secret} />
				{instance && secretRequired ? (
					<span className="text-xs text-muted-foreground">{NEW_KEY_HELP}</span>
				) : null}
				{instance && !secretRequired ? (
					<span className="text-xs text-muted-foreground">
						Laissez vide pour garder la clé actuelle ({instance.secret_preview}
						).
					</span>
				) : null}
				{values.kind !== 'n8n' ? (
					<span className="text-xs text-muted-foreground">{ODOO_HELP}</span>
				) : null}
			</div>
			{errorMessage ? (
				<InlineAlert tone="danger">{errorMessage}</InlineAlert>
			) : null}
			<DialogFooter>
				<Button
					type="button"
					variant="ghost"
					disabled={isPending}
					onClick={onClose}
				>
					Annuler
				</Button>
				<Button type="submit" disabled={isPending}>
					{isPending
						? 'Test de la connexion…'
						: instance
							? 'Enregistrer'
							: 'Ajouter'}
				</Button>
			</DialogFooter>
		</form>
	)
}

export function IntegrationInstanceDialog({ open, ...form }: Props) {
	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (!next && !form.isPending) form.onClose()
			}}
		>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						{form.instance
							? 'Modifier l’intégration'
							: 'Ajouter une intégration'}
					</DialogTitle>
					<DialogDescription>
						Une instance Odoo ou n8n de ce client. La connexion est testée avant
						l’enregistrement.
					</DialogDescription>
				</DialogHeader>
				<IntegrationForm key={form.instance?.id ?? 'new'} {...form} />
			</DialogContent>
		</Dialog>
	)
}
