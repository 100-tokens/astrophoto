//! Emits TypeScript types to the frontend. Invoked via `just types`.

use std::fs;
use std::path::Path;
use ts_rs::TS;

use astrophoto::api_types::{
    AuthError, Health, Preferences, Profile, SessionRow, User, UserPublic,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("../frontend/src/lib/api");
    fs::create_dir_all(out_dir)?;

    // export_all_to writes each type (from #[ts(export_to = "Foo.ts")])
    // into the given directory, together with all transitive dependencies.
    Health::export_all_to(out_dir)?;
    User::export_all_to(out_dir)?;
    AuthError::export_all_to(out_dir)?;
    UserPublic::export_all_to(out_dir)?;
    Profile::export_all_to(out_dir)?;
    Preferences::export_all_to(out_dir)?;
    SessionRow::export_all_to(out_dir)?;

    println!("Wrote types to: {}", out_dir.display());
    Ok(())
}
