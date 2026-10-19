use capitainerie_common::CoreError;
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum GithubError {
    #[error("github transport error: {0}")]
    Transport(String),

    #[error("github returned {status}: {message}")]
    Status { status: u16, message: String },

    #[error("cannot sign the app token: {0}")]
    Jwt(String),

    #[error("unusable GitHub App: {0}")]
    InvalidApp(String),

    #[error("unexpected github response: {0}")]
    Decode(String),

    /// The App lacks a permission the call needs; GitHub answers 403
    /// « Resource not accessible by integration ».
    #[error("the GitHub App lacks the {0} permission")]
    MissingPermission(&'static str),
}

impl GithubError {
    pub(crate) fn transport(error: reqwest::Error) -> Self {
        Self::Transport(error.without_url().to_string())
    }

    pub(crate) fn is_transient(&self) -> bool {
        match self {
            Self::Transport(_) | Self::Jwt(_) | Self::Decode(_) => true,
            Self::InvalidApp(_) | Self::MissingPermission(_) => false,
            Self::Status { status, .. } => {
                *status == StatusCode::TOO_MANY_REQUESTS.as_u16() || *status >= 500
            }
        }
    }
}

impl From<GithubError> for CoreError {
    fn from(error: GithubError) -> Self {
        if error.is_transient() {
            tracing::warn!(%error, "github call failed, transient");
            CoreError::Internal(error.to_string())
        } else {
            CoreError::Invalid(error.to_string())
        }
    }
}
