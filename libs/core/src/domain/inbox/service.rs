use capitainerie_common::{CoreError, InboxItemId};
use chrono::Utc;

use super::{InboxPage, ReadSelection, ports::InboxRepository};
use crate::domain::authorization::{
    AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require,
};

pub struct InboxService<'a, R, A> {
    items: R,
    authorizer: &'a A,
}

impl<'a, R: InboxRepository, A: Authorizer> InboxService<'a, R, A> {
    pub fn new(items: R, authorizer: &'a A) -> Self {
        Self { items, authorizer }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        unread_only: bool,
        before: Option<InboxItemId>,
        limit: u32,
    ) -> Result<InboxPage, CoreError> {
        let organization_id = subject.organization_id;
        let member = subject.id.clone();
        self.require(subject).await?;
        let mut items = self
            .items
            .list(organization_id, &member, unread_only, before, limit + 1)
            .await?;
        let next_cursor = if items.len() > limit as usize {
            items.truncate(limit as usize);
            items.last().map(|item| item.id)
        } else {
            None
        };
        let (total, unread) = self.items.counts(organization_id, &member).await?;
        Ok(InboxPage {
            items,
            next_cursor,
            total,
            unread,
        })
    }

    /// Returns `(updated, unread)`.
    pub async fn mark_read(
        mut self,
        subject: Subject,
        selection: ReadSelection,
    ) -> Result<(u64, i64), CoreError> {
        let organization_id = subject.organization_id;
        let member = subject.id.clone();
        self.require(subject).await?;
        let updated = self
            .items
            .mark_read(organization_id, &member, &selection, Utc::now())
            .await?;
        let (_, unread) = self.items.counts(organization_id, &member).await?;
        Ok((updated, unread))
    }

    async fn require(&self, subject: Subject) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, Action::InboxRead, ResourceKind::Inbox, None),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;
    use uuid::Uuid;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        inbox::{InboxItem, InboxKind, Notification, ports::MockInboxRepository},
    };

    fn subject() -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: OrganizationId::new(),
            actor: None,
            is_admin: false,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn item() -> InboxItem {
        InboxItem::project(
            OrganizationId::new(),
            Uuid::now_v7(),
            Notification {
                member_subject: "user-1".into(),
                kind: InboxKind::Card,
                title: "t".into(),
                body: String::new(),
                link: None,
                card_id: None,
                run_id: None,
            },
            Utc::now(),
        )
    }

    #[tokio::test]
    async fn a_page_reads_the_member_s_own_items_and_counts() {
        let items = vec![item(), item(), item()];
        let cursor = items[1].id;
        let mut repository = MockInboxRepository::new();
        repository
            .expect_list()
            .withf(|_, member, unread, _, limit| member == "user-1" && *unread && *limit == 3)
            .returning(move |_, _, _, _, _| {
                let items = items.clone();
                Box::pin(async move { Ok(items) })
            });
        repository
            .expect_counts()
            .returning(|_, _| Box::pin(async { Ok((67, 5)) }));
        let authorizer = allow();
        let page = InboxService::new(repository, &authorizer)
            .list(subject(), true, None, 2)
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_cursor, Some(cursor));
        assert_eq!((page.total, page.unread), (67, 5));
    }

    #[tokio::test]
    async fn marking_read_returns_what_is_left() {
        let mut repository = MockInboxRepository::new();
        repository
            .expect_mark_read()
            .withf(|_, member, selection, _| member == "user-1" && *selection == ReadSelection::All)
            .returning(|_, _, _, _| Box::pin(async { Ok(4) }));
        repository
            .expect_counts()
            .returning(|_, _| Box::pin(async { Ok((10, 0)) }));
        let authorizer = allow();
        let (updated, unread) = InboxService::new(repository, &authorizer)
            .mark_read(subject(), ReadSelection::All)
            .await
            .unwrap();
        assert_eq!((updated, unread), (4, 0));
    }
}
