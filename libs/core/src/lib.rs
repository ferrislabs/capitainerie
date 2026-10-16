extern crate self as capitainerie_core;

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::CapitainerieUseCase;
pub use domain::organization::{Organization, OrganizationSlug};
