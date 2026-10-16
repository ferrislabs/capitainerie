use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, ItemFn, ItemStruct, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/// Registers a repository for a `(domain, backend)` pair, so `#[transactional]`
/// can build it from a transaction.
///
/// ```ignore
/// #[repository(domain = Organization, backend = Postgres)]
/// pub struct PgOrganizationRepository<'tx> { tx: SharedTx<'tx> }
/// ```
#[proc_macro_attribute]
pub fn repository(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(args as RepositoryAttrs);
    let item = parse_macro_input!(input as ItemStruct);

    let name = &item.ident;
    let domain = &attrs.domain;
    let backend = &attrs.backend;

    quote! {
        #item

        impl ::capitainerie_core::infrastructure::registry::RepoFor<
            ::capitainerie_core::infrastructure::registry::domain::#domain
        > for ::capitainerie_core::infrastructure::registry::backend::#backend {
            type Repo<'tx> = #name<'tx>;

            fn build<'tx>(
                tx: &::capitainerie_core::infrastructure::postgres::SharedTx<'tx>,
            ) -> Self::Repo<'tx> {
                #name::new(tx)
            }
        }
    }
    .into()
}

/// Runs the method body in one transaction and binds `<name>_repository` for
/// each listed domain. Commits on `Ok`, rolls back on `Err`.
///
/// ```ignore
/// #[transactional(organization)]
/// pub async fn organization_by_issuer(&self, issuer: &str) -> Result<Organization, CoreError> {
///     OrganizationService::new(organization_repository).by_issuer(issuer).await
/// }
/// ```
#[proc_macro_attribute]
pub fn transactional(args: TokenStream, input: TokenStream) -> TokenStream {
    let domains = parse_macro_input!(args as DomainList);
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = parse_macro_input!(input as ItemFn);

    let bindings = domains.names.iter().map(|name| {
        let binding = Ident::new(&format!("{name}_repository"), name.span());
        let domain = Ident::new(&pascal_case(&name.to_string()), name.span());
        quote! {
            let #binding = <
                ::capitainerie_core::infrastructure::registry::Backend
                as ::capitainerie_core::infrastructure::registry::RepoFor<
                    ::capitainerie_core::infrastructure::registry::domain::#domain
                >
            >::build(&tx);
        }
    });
    let statements = &block.stmts;

    quote! {
        #(#attrs)*
        #vis #sig {
            ::capitainerie_core::infrastructure::postgres::with_tx(&self.pool, async |tx| {
                #(#bindings)*
                #(#statements)*
            })
            .await
        }
    }
    .into()
}

struct RepositoryAttrs {
    domain: Ident,
    backend: Ident,
}

impl Parse for RepositoryAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut domain = None;
        let mut backend = None;
        for KeyValue { key, value } in Punctuated::<KeyValue, Token![,]>::parse_terminated(input)? {
            match key.to_string().as_str() {
                "domain" => domain = Some(value),
                "backend" => backend = Some(value),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown key `{other}`, expected `domain` or `backend`"),
                    ));
                }
            }
        }
        Ok(Self {
            domain: domain.ok_or_else(|| syn::Error::new(input.span(), "missing `domain`"))?,
            backend: backend.ok_or_else(|| syn::Error::new(input.span(), "missing `backend`"))?,
        })
    }
}

struct KeyValue {
    key: Ident,
    value: Ident,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self { key, value })
    }
}

struct DomainList {
    names: Vec<Ident>,
}

impl Parse for DomainList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let names = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            names: names.into_iter().collect(),
        })
    }
}

fn pascal_case(snake: &str) -> String {
    snake
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}
