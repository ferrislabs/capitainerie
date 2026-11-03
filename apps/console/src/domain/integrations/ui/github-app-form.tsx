import { Github } from 'lucide-react'
import type { FormEvent } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
import { RadioGroup, RadioGroupItem } from '#/components/ui/radio-group'

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
			<Field label="Propriétaire de l’application" group>
				<RadioGroup
					value={ownerKind}
					onValueChange={(kind) => onOwnerKindChange(kind as OwnerKind)}
					disabled={isPending}
				>
					<RadioGroupItem value="user" label="Compte personnel" />
					<RadioGroupItem value="organization" label="Organisation GitHub" />
				</RadioGroup>
			</Field>
			{ownerKind === 'organization' ? (
				<Field
					label="Login de l’organisation GitHub"
					id="github-owner-login"
					className="w-full max-w-sm"
				>
					<Input
						autoComplete="off"
						spellCheck={false}
						className="font-mono"
						placeholder="acme"
						value={login}
						onChange={(event) => onLoginChange(event.target.value)}
						disabled={isPending}
						invalid={login !== '' && !valid}
					/>
				</Field>
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
