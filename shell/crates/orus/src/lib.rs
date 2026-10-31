//! `orus`: install, update and remove Orus OS web apps, from a folder, an archive, a
//! URL or a signed catalog.
//!
//! A *source* is a catalog on a server: `index.json` listing apps, with a detached
//! Ed25519 signature `index.json.sig`. An app is a `.orusapp` file (a zip of the built
//! bundle). The catalog gives each app's SHA-256, and nothing is unpacked until the
//! download matches it.

pub mod archive;
pub mod catalog;
pub mod fetch;
pub mod serve;
pub mod sign;
pub mod sources;
pub mod store;
