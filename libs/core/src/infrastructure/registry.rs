//! Maps a (domain, backend) pair to its repository. Each `Pg*Repository`
//! registers itself with `#[repository(domain = X, backend = Postgres)]`, and
//! `#[transactional]` resolves `<Backend as RepoFor<domain::X>>::Repo`.

use crate::infrastructure::postgres::SharedTx;

pub mod domain {
    pub struct Approval;
    pub struct Card;
    pub struct CardComment;
    pub struct Client;
    pub struct Conversation;
    pub struct CrewAgent;
    pub struct ConversationMessage;
    pub struct DecisionLog;
    pub struct GithubApp;
    pub struct GithubInstallation;
    pub struct McpConnector;
    pub struct Member;
    pub struct ModelAssignment;
    pub struct Organization;
    pub struct Project;
    pub struct Routine;
    pub struct RoutineRun;
    pub struct Run;
    pub struct Skill;
    pub struct Source;
    pub struct SourceEvent;
    pub struct Step;
    pub struct TaskGrant;
    pub struct Transcript;
    pub struct Worksite;
}

pub mod backend {
    pub struct Postgres;
}

pub type Backend = backend::Postgres;

pub trait RepoFor<D> {
    type Repo<'tx>;

    fn build<'tx>(tx: &SharedTx<'tx>) -> Self::Repo<'tx>;
}
