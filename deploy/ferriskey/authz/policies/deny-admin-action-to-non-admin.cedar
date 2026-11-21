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
when {
  !context.is_admin
};
