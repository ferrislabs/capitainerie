@id("human-acts-in-own-organization")
permit (
  principal,
  action in [
    Capitainerie::Action::"transcript.create",
    Capitainerie::Action::"transcription.create",
    Capitainerie::Action::"document.read",
    Capitainerie::Action::"run.read",
    Capitainerie::Action::"card.create",
    Capitainerie::Action::"card.read",
    Capitainerie::Action::"card.comment",
    Capitainerie::Action::"provider.read",
    Capitainerie::Action::"user_provider.manage",
    Capitainerie::Action::"card.execute",
    Capitainerie::Action::"client.read",
    Capitainerie::Action::"client.manage",
    Capitainerie::Action::"member.read",
    Capitainerie::Action::"github.read",
    Capitainerie::Action::"conversation.read",
    Capitainerie::Action::"conversation.write",
    Capitainerie::Action::"integration.read",
    Capitainerie::Action::"card.intervene",
    Capitainerie::Action::"integration.revert",
    Capitainerie::Action::"agent.read",
    Capitainerie::Action::"agent.manage",
    Capitainerie::Action::"mcp_connector.manage",
    Capitainerie::Action::"mcp_connector.connect",
    Capitainerie::Action::"agent.run",
    Capitainerie::Action::"skill.read",
    Capitainerie::Action::"skill.manage",
    Capitainerie::Action::"approval.read",
    Capitainerie::Action::"approval.decide",
    Capitainerie::Action::"source.read",
    Capitainerie::Action::"source.manage",
    Capitainerie::Action::"routine.read",
    Capitainerie::Action::"routine.manage",
    Capitainerie::Action::"audit.read",
    Capitainerie::Action::"inbox.read",
    Capitainerie::Action::"artifact.read",
    Capitainerie::Action::"integration.call"
  ],
  resource
)
when {
  context.actor_kind == "human" && !(context has actor) &&
  context.organization == resource.organization
};
