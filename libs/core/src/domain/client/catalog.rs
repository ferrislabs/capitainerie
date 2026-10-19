use capitainerie_common::{CoreError, OrganizationId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{Client, links::Links, ports::ClientRepository};
use crate::domain::project::{Project, ports::ProjectRepository};

pub const SHORT_ID_CHARS: usize = 8;

pub fn short_id(id: Uuid) -> String {
    id.simple()
        .to_string()
        .chars()
        .take(SHORT_ID_CHARS)
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    clients: Vec<Client>,
    projects: Vec<Project>,
}

enum Entry {
    Client(Client),
    Project(Project),
}

impl Catalog {
    pub const MAX_ENTRIES: usize = 200;

    pub fn new(
        clients: Vec<(DateTime<Utc>, Client)>,
        projects: Vec<(DateTime<Utc>, Project)>,
    ) -> Self {
        let mut entries: Vec<(DateTime<Utc>, Entry)> = clients
            .into_iter()
            .map(|(used_at, client)| (used_at, Entry::Client(client)))
            .chain(
                projects
                    .into_iter()
                    .map(|(used_at, project)| (used_at, Entry::Project(project))),
            )
            .collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        entries.truncate(Self::MAX_ENTRIES);
        let mut catalog = Self::default();
        for (_, entry) in entries {
            match entry {
                Entry::Client(client) => catalog.clients.push(client),
                Entry::Project(project) => catalog.projects.push(project),
            }
        }
        catalog
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty() && self.projects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.clients.len() + self.projects.len()
    }

    pub fn block(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let mut sections = Vec::new();
        if !self.clients.is_empty() {
            let lines = self
                .clients
                .iter()
                .map(|client| line(client.id.0, client.name.as_str(), client.aliases.as_slice()));
            sections.push(section("Clients connus (id court : nom)", lines));
        }
        if !self.projects.is_empty() {
            let lines = self.projects.iter().map(|project| {
                line(
                    project.id.0,
                    project.name.as_str(),
                    project.aliases.as_slice(),
                )
            });
            sections.push(section("Projets connus (id court : nom)", lines));
        }
        Some(sections.join("\n"))
    }

    pub fn resolve(&self, client_ref: Option<&str>, project_ref: Option<&str>) -> Links {
        let project = project_ref
            .and_then(|reference| unique(&self.projects, reference, |project| project.id.0));
        if let Some(project) = project
            && let Ok(links) = Links::of(None, Some(project))
        {
            return links;
        }
        client_ref
            .and_then(|reference| unique(&self.clients, reference, |client| client.id.0))
            .and_then(|client| Links::of(Some(client.reference()), None).ok())
            .unwrap_or_default()
    }
}

fn line(id: Uuid, name: &str, aliases: &[String]) -> String {
    if aliases.is_empty() {
        format!("- {} : {name}", short_id(id))
    } else {
        format!(
            "- {} : {name} (alias : {})",
            short_id(id),
            aliases.join(", ")
        )
    }
}

fn section(title: &str, lines: impl Iterator<Item = String>) -> String {
    let mut section = format!("{title} :");
    for line in lines {
        section.push('\n');
        section.push_str(&line);
    }
    section
}

fn unique<'a, T>(items: &'a [T], reference: &str, id: impl Fn(&T) -> Uuid) -> Option<&'a T> {
    let wanted = reference.trim().trim_start_matches('#').to_lowercase();
    if wanted.len() != SHORT_ID_CHARS || !wanted.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut matching = items.iter().filter(|item| short_id(id(item)) == wanted);
    let found = matching.next()?;
    matching.next().is_none().then_some(found)
}

pub async fn load<C: ClientRepository, P: ProjectRepository>(
    clients: &mut C,
    projects: &mut P,
    organization_id: OrganizationId,
) -> Result<Catalog, CoreError> {
    let limit = i64::try_from(Catalog::MAX_ENTRIES).unwrap_or(i64::MAX);
    Ok(Catalog::new(
        clients.recent(organization_id, limit).await?,
        projects.recent(organization_id, limit).await?,
    ))
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{ClientId, ProjectId};
    use chrono::Duration;

    use super::*;
    use crate::domain::client::links::tests::{client, project};

    fn ago(minutes: i64) -> DateTime<Utc> {
        Utc::now() - Duration::minutes(minutes)
    }

    #[test]
    fn the_short_id_is_the_first_eight_hex_characters() {
        let id = Uuid::parse_str("1a2b3c4d-5e6f-4a8b-9c0d-1e2f3a4b5c6d").unwrap();
        assert_eq!(short_id(id), "1a2b3c4d");
    }

    #[test]
    fn an_empty_catalog_has_no_block() {
        assert_eq!(Catalog::default().block(), None);
    }

    #[test]
    fn the_block_lists_short_id_name_and_aliases() {
        let org = OrganizationId::new();
        let mut acme = client(org, "Acme");
        acme.aliases = crate::domain::label::Aliases::try_from(vec!["ACME SA".into()]).unwrap();
        let site = project(org, "Site", Some(&acme));
        let block = Catalog::new(vec![(ago(1), acme.clone())], vec![(ago(2), site.clone())])
            .block()
            .unwrap();
        assert!(block.contains(&format!(
            "- {} : Acme (alias : ACME SA)",
            short_id(acme.id.0)
        )));
        assert!(block.contains(&format!("- {} : Site", short_id(site.id.0))));
        assert!(block.starts_with("Clients connus"));
    }

    #[test]
    fn it_keeps_the_two_hundred_most_recently_used() {
        let org = OrganizationId::new();
        let clients: Vec<_> = (0..150)
            .map(|n| (ago(n), client(org, &format!("C{n}"))))
            .collect();
        let projects: Vec<_> = (0..150)
            .map(|n| (ago(1_000 + n), project(org, &format!("P{n}"), None)))
            .collect();
        let catalog = Catalog::new(clients, projects);
        assert_eq!(catalog.len(), Catalog::MAX_ENTRIES);
        assert_eq!(catalog.clients.len(), 150);
        assert_eq!(catalog.projects.len(), 50);
        assert_eq!(catalog.clients[0].name.as_str(), "C0");
    }

    #[test]
    fn a_known_reference_resolves_and_the_project_brings_its_client() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let site = project(org, "Site", Some(&acme));
        let catalog = Catalog::new(vec![(ago(1), acme.clone())], vec![(ago(2), site.clone())]);
        let by_project = catalog.resolve(None, Some(&short_id(site.id.0)));
        assert_eq!(by_project.project_id(), Some(site.id));
        assert_eq!(by_project.client_id(), Some(acme.id));
        let by_client = catalog.resolve(Some(&short_id(acme.id.0).to_uppercase()), None);
        assert_eq!(by_client.client_id(), Some(acme.id));
        assert_eq!(by_client.project(), None);
    }

    #[test]
    fn an_unknown_reference_is_ignored_and_nothing_is_created() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let catalog = Catalog::new(vec![(ago(1), acme)], vec![]);
        for reference in ["deadbeef", "", "acme", "1234", "zzzzzzzz", "00000000-0000"] {
            assert!(catalog.resolve(Some(reference), Some(reference)).is_empty());
        }
    }

    #[test]
    fn an_ambiguous_reference_is_ignored() {
        let org = OrganizationId::new();
        let mut first = client(org, "Acme");
        let mut second = client(org, "Globex");
        first.id = ClientId(Uuid::parse_str("aaaaaaaa-0000-4000-8000-000000000001").unwrap());
        second.id = ClientId(Uuid::parse_str("aaaaaaaa-0000-4000-8000-000000000002").unwrap());
        let catalog = Catalog::new(vec![(ago(1), first), (ago(2), second)], vec![]);
        assert!(catalog.resolve(Some("aaaaaaaa"), None).is_empty());
    }

    #[test]
    fn a_client_reference_does_not_resolve_as_a_project_and_the_project_wins_a_disagreement() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let globex = client(org, "Globex");
        let site = project(org, "Site", Some(&acme));
        let catalog = Catalog::new(
            vec![(ago(1), acme.clone()), (ago(2), globex.clone())],
            vec![(ago(3), site.clone())],
        );
        assert!(catalog.resolve(None, Some(&short_id(acme.id.0))).is_empty());
        let links = catalog.resolve(Some(&short_id(globex.id.0)), Some(&short_id(site.id.0)));
        assert_eq!(links.client_id(), Some(acme.id));
        assert_eq!(links.project_id(), Some(ProjectId(site.id.0)));
    }
}
