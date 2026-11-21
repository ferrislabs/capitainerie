@id("deny-non-human-moving-into-proposed-or-rejected")
forbid (
  principal,
  action == Capitainerie::Action::"card.move",
  resource
)
when {
  context.actor_kind != "human" &&
  ["proposed", "rejected"].contains(context.to)
};
