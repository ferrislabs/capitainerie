@id("human-administers-own-organization")
permit (
  principal,
  action in [
    Capitainerie::Action::"provider.manage",
    Capitainerie::Action::"models.assign",
    Capitainerie::Action::"github.manage",
    Capitainerie::Action::"integration.manage",
    Capitainerie::Action::"budget.manage"
  ],
  resource
)
when {
  context.actor_kind == "human" && !(context has actor) &&
  context.organization == resource.organization &&
  context.is_admin
};
