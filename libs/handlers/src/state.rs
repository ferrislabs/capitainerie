use capitainerie_auth::{FerrisKeyTokenExchange, FerrisKeyVerifier};
use capitainerie_core::CapitainerieUseCase;

pub type CapitainerieTokenVerifier = FerrisKeyVerifier;
pub type CapitainerieTokenExchanger = FerrisKeyTokenExchange;

#[derive(Clone)]
pub struct AppState {
    pub usecase: CapitainerieUseCase,
    pub verifier: CapitainerieTokenVerifier,
    /// `None` when the API has no exchange credentials: runs are then queued
    /// without a delegation and their write step fails, saying so.
    pub delegation: Option<Delegation>,
}

#[derive(Clone)]
pub struct Delegation {
    pub exchanger: CapitainerieTokenExchanger,
    /// Client ids of the agents a run acts through.
    pub agents: Vec<String>,
}
