@id("captain-reads-proposes-and-comments")
permit (
  principal,
  action in [
    Capitainerie::Action::"card.create",
    Capitainerie::Action::"card.read",
    Capitainerie::Action::"card.comment",
    Capitainerie::Action::"client.read",
    Capitainerie::Action::"github.read"
  ],
  resource
)
when { context.actor_kind == "captain" };

@id("captain-links-cards")
permit (principal, action == Capitainerie::Action::"card.update", resource)
when { context.actor_kind == "captain" && context.intent == "link" };
