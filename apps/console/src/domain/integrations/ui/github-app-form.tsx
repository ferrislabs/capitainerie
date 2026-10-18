import { Github } from 'lucide-react'
import type { FormEvent } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Input } from '#/components/ui/input'
import { Label } from '#/components/ui/label'

export type OwnerKind = 'user' | 'organization'

const LOGIN_PATTERN = /^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$/

export function isValidOwner(kind: OwnerKind, login: string): boolean {
	return kind === 'user' || LOGIN_PATTERN.test(login.trim())
}

interface Props {
	ownerKind: OwnerKind
	login: string
	isPending: boolean
	errorMessage?: string
	onOwnerKindChange: (kind: OwnerKind) => void
	onLoginChange: (login: string) => void
	onSubmit: () => void
}

export function GithubAppForm({
	ownerKind,
	login,
	isPending,
	errorMessage,
	onOwnerKindChange,
	onLoginChange,
	onSubmit,
}: Props) {
	const valid = isValidOwner(ownerKind, login)

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		if (valid && !isPending) onSubmit()
	}

	return (
		<form
			onSubmit={handleSubmit}
			noValidate
			className="flex flex-col items-start gap-4"
		>
			<p className="text-sm text-muted-foreground">
				Claude Code agit sur vos dépôts par une application GitHub propre à
				l’organisation. GitHub vous demandera de la valider ; aucune clé n’est à
				copier.
			</p>
			<fieldset className="flex flex-col gap-2">
				<legend className="mb-1 text-sm font-medium">
					Propriétaire de l’application
				</legend>
				{(
					[
						['user', 'Compte personnel'],
						['organization', 'Organisation GitHub'],
					] as const
				).map(([kind, label]) => (
					<label key={kind} className="flex items-center gap-2 text-sm">
						<input
							type="radio"
							name="github-app-owner"
							value={kind}
							checked={ownerKind === kind}
							disabled={isPending}
							onChange={() => onOwnerKindChange(kind)}
							className="size-4 accent-primary"
						/>
						{label}
					</label>
				))}
			</fieldset>
			{ownerKind === 'organization' ? (
				<div className="flex w-full max-w-sm flex-col gap-1.5">
					<Label htmlFor="github-owner-login">
						Login de l’organisation GitHub
					</Label>
					<Input
						id="github-owner-login"
						autoComplete="off"
						spellCheck={false}
						className="font-mono"
						placeholder="acme"
						value={login}
						onChange={(event) => onLoginChange(event.target.value)}
						disabled={isPending}
						aria-invalid={login !== '' && !valid}
					/>
				</div>
			) : null}
			<Button type="submit" disabled={isPending || !valid}>
				<Github aria-hidden="true" />
				{isPending ? 'Préparation…' : 'Créer l’application GitHub'}
			</Button>
			{errorMessage ? (
				<InlineAlert tone="danger" className="w-full">
					{errorMessage}
				</InlineAlert>
			) : null}
		</form>
	)
}
