/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Shared font database for text rendering.
//!
//! usvg needs a `fontdb::Database` to lay out `<text>`; with the default (empty)
//! db, text is silently dropped. We load the system fonts once and share the
//! `Arc` across every render — loading is comparatively expensive, so it must not
//! happen per frame.

use std::sync::{Arc, OnceLock};

use usvg::fontdb::Database;

static FONTDB: OnceLock<Arc<Database>> = OnceLock::new();

/// A process-wide font database with the system fonts loaded. Cheap to call
/// repeatedly — the first call populates it, later calls clone the `Arc`.
pub fn shared() -> Arc<Database> {
    FONTDB
        .get_or_init(|| {
            let mut db = Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}
