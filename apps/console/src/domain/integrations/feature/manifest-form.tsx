import { useEffect, useRef } from 'react'

interface Props {
	actionUrl: string
	manifest: string
}

export function ManifestForm({ actionUrl, manifest }: Props) {
	const form = useRef<HTMLFormElement>(null)
	const submitted = useRef(false)

	useEffect(() => {
		if (submitted.current) return
		submitted.current = true
		form.current?.submit()
	}, [])

	return (
		<form
			ref={form}
			method="post"
			action={actionUrl}
			hidden
			data-testid="github-manifest-form"
		>
			<input type="hidden" name="manifest" value={manifest} />
		</form>
	)
}
