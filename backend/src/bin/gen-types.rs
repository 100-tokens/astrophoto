//! Emits TypeScript types to the frontend. Invoked via `just types`.

use std::fs;
use std::path::Path;
use ts_rs::TS;

use astrophoto::api_types::Health;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("../frontend/src/lib/api");
    fs::create_dir_all(out_dir)?;

    // export_all_to writes Health.ts (from #[ts(export_to = "Health.ts")])
    // into the given directory, together with all transitive dependencies.
    Health::export_all_to(out_dir)?;

    println!("Wrote: {}/Health.ts", out_dir.display());
    Ok(())
}
