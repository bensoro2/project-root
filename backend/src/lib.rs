//! Shared library for backend crate
//! Provides modules for handlers, storage, embedding, etc. so that they can be
//! reused by multiple binary targets (e.g. `backend`, `index_builder`).

pub mod embed;
pub mod error;
pub mod handlers;
pub mod storage;
#[cfg(feature = "fastembed")]
pub mod bulk_insert;
