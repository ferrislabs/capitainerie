@id("deny-cross-organization")
forbid (
  principal,
  action in Capitainerie::Action::"capitainerie.any",
  resource
)
when {
  context.organization != resource.organization
};
