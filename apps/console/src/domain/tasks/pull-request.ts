const PULL_REQUEST = /https:\/\/github\.com\/([\w.-]+)\/([\w.-]+)\/pull\/(\d+)/

export interface PullRequestLink {
	url: string
	repository: string
	number: number
}

/** La première pull request GitHub qu'un commentaire d'exécution cite. */
export function pullRequestLink(body: string): PullRequestLink | null {
	const match = PULL_REQUEST.exec(body)
	if (!match) return null
	return {
		url: match[0],
		repository: `${match[1]}/${match[2]}`,
		number: Number(match[3]),
	}
}
