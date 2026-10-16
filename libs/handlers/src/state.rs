use capitainerie_auth::FerrisKeyVerifier;
use capitainerie_core::CapitainerieUseCase;

pub type CapitainerieTokenVerifier = FerrisKeyVerifier;

#[derive(Clone)]
pub struct AppState {
    pub usecase: CapitainerieUseCase,
    pub verifier: CapitainerieTokenVerifier,
}
