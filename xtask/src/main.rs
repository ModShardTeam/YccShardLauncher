use std::{
    env,
    fs::{File, create_dir_all, rename},
    iter,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result, bail};
use zip::write::SimpleFileOptions;

const PRODUCED_FILES: [&str; 2] = ["hook.dll", "injector.exe"];
const AUXILIARY_FILES: [&str; 2] = ["LICENSE", "README.md"];

fn main() -> Result<()> {
    println!("Starting xtask");
    let main_dir = env!("CARGO_WORKSPACE_DIR");
    let cargo = std::env::var("CARGO").context("run this through cargo xtask")?;

    let status = Command::new(cargo)
        .args(["build", "--release", "-p", "hook", "-p", "injector"])
        .current_dir(main_dir)
        .status()?;

    if !status.success() {
        bail!("build failed with status {}", status);
    }

    let zip_dir = PathBuf::from(main_dir).join("bin");
    create_dir_all(&zip_dir)?;

    let dst = zip_dir.join(format!("YSL-{}.zip", env!("CARGO_PKG_VERSION")));
    let tmp_dst = zip_dir.join("tmp.zip");
    let tmp_file = File::create(&tmp_dst)?;
    let mut zip = zip::ZipWriter::new(tmp_file);
    let options = SimpleFileOptions::default();

    let work = iter::zip(
        PRODUCED_FILES,
        iter::repeat(PathBuf::from(main_dir).join("target/release")),
    )
    .chain(iter::zip(
        AUXILIARY_FILES,
        iter::repeat(PathBuf::from(main_dir)),
    ));

    for (entry_name, dir) in work {
        let entry = dir.join(entry_name);

        if !entry.is_file() {
            bail!("{:?} is not a valid file", entry);
        }

        println!("Zipping {:?}", entry);
        zip.start_file(entry_name, options)?;
        let mut f = File::open(entry)?;
        std::io::copy(&mut f, &mut zip)?;
    }

    zip.finish()?;
    println!("Renaming {:?} as {:?}", tmp_dst, dst);
    rename(tmp_dst, &dst)?;

    Ok(())
}
