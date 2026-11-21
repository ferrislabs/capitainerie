@id("registry-agent-reads-and-comments")
permit (
  principal,
  action in [
    Capitainerie::Action::"card.read",
    Capitainerie::Action::"card.comment"
  ],
  resource
)
when {
  context.actor_kind == "registry" && context has actor &&
  context.organization == resource.organization
};
