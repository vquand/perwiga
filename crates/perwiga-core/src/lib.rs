//! Shared local-first application contracts and persistence for Perwiga.

pub mod error;
pub mod feed;
pub mod http;
pub mod model;
pub mod module;
pub mod rich_text;
pub mod service;
pub mod storage;

pub use error::{PerwigaError, Result};
pub use module::{
    CalendarEventPresentation, Capability, EntityEventRecencyPresentation, EntityFacetDefinition,
    EntityFacetOption, EntityPresentation, EntityTypeDefinition, EventFeaturedEntityPresentation,
    EventPreviousGapPresentation, LibraryModule, ModuleRegistry, ThemeDefinition, WorkKind,
};
pub use storage::Store;
