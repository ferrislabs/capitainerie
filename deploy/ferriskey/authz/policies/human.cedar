@id("human-acts-in-own-organization")
permit (principal, action in Capitainerie::Action::"capitainerie.any", resource)
when { context.actor_kind == "human" };
