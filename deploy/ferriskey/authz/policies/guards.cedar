@id("deny-cross-organization")
forbid (principal, action in Capitainerie::Action::"capitainerie.any", resource)
when { context.organization != resource.organization };

@id("deny-admin-action-to-non-admin")
forbid (
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
when { !context.is_admin };

@id("deny-mcp-call-without-initiator-and-grant")
forbid (principal, action == Capitainerie::Action::"mcp.call", resource)
unless { context.initiated && context.granted };

@id("deny-non-human-leaving-proposed")
forbid (principal, action == Capitainerie::Action::"card.move", resource)
when { context.actor_kind != "human" && context.from == "proposed" };

@id("deny-non-human-moving-into-proposed-or-rejected")
forbid (principal, action == Capitainerie::Action::"card.move", resource)
when { context.actor_kind != "human" && ["proposed", "rejected"].contains(context.to) };
