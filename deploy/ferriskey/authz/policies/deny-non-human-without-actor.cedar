@id("deny-non-human-without-actor")
forbid (
  principal,
  action in Capitainerie::Action::"capitainerie.any",
  resource
)
when {
  context.actor_kind != "human" && !(context has actor)
};
