import { useNavigate } from '@tanstack/react-router'
import { useCallback, useState } from 'react'
import { chatErrorMessage } from '#/domain/chat/chat-errors'
import { useCaptainModel } from '#/domain/chat/hooks/use-captain-model'
import { useCreateConversation } from '#/domain/chat/hooks/use-create-conversation'
import { useVoiceInput } from '#/domain/chat/hooks/use-voice-input'

export function appendDictation(current: string, text: string): string {
	return current.trim() === '' ? text : `${current.trimEnd()} ${text}`
}

export function useNewConversation() {
	const navigate = useNavigate()
	const create = useCreateConversation()
	const captain = useCaptainModel()
	const [value, setValue] = useState('')
	const voice = useVoiceInput(
		useCallback(
			(text: string) => setValue((current) => appendDictation(current, text)),
			[],
		),
	)
	const mutate = create.mutate

	const submit = useCallback(() => {
		const message = value.trim()
		if (message === '' || create.isPending) return
		mutate(
			{ body: { message } },
			{
				onSuccess: (created) =>
					void navigate({ to: '/chat/$id', params: { id: created.id } }),
			},
		)
	}, [value, create.isPending, mutate, navigate])

	return {
		value,
		setValue,
		submit,
		isPending: create.isPending,
		errorMessage: create.error ? chatErrorMessage(create.error) : undefined,
		voice,
		captain,
	}
}
