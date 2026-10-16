use std::fs;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;
use dll_syringe::{Syringe, process::OwnedProcess};
use windows::Win32::Foundation::{HANDLE, WAIT_FAILED, WAIT_TIMEOUT};
use windows::Win32::System::Threading::WaitForInputIdle;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    game: PathBuf,
}

const DLL_NAME: &str = "hook.dll";

fn main() -> Result<()> {
    let cli = Cli::parse();

    let game_path = fs::canonicalize(cli.game)?;
    let parent = game_path
        .parent()
        .with_context(|| format!("Cannot find parent of {:?}", game_path))?;

    if !game_path.is_file() {
        // TODO: check if it's a correct .exe file with maybe a .win in the same folder
        // TODO: add maybe some more checks
        bail!("Not a valid exe {:#?}", game_path);
    }

    // copy dll into the game folder
    let dll_src = std::env::current_exe()?
        .parent()
        .with_context(|| "Cannot find parent of the current exe")?
        .join(DLL_NAME);
    let dll_dst = parent.join(DLL_NAME);
    let log_dir = parent
        .to_str()
        .with_context(|| format!("Couldnt parse {:?} to string", parent))?;
    println!("Copying from {:?} to {:?}", dll_src, dll_dst);
    fs::copy(dll_src, &dll_dst)?;

    // launch game
    let child = Command::new(&game_path).current_dir(parent).spawn()?;

    // get process
    let process: OwnedProcess = OwnedProcess::from_pid(child.id())?;
    let syringe = Syringe::for_process(process);

    // wait for process to be initialized
    let wait = unsafe { WaitForInputIdle(HANDLE(child.as_raw_handle() as _), 30_000) };
    if wait == WAIT_FAILED.0 {
        bail!("wait failed");
    } else if wait == WAIT_TIMEOUT.0 {
        println!("wait stopped because of timeout");
    } else if wait == 0 {
        println!("wait succeeded");
    }

    // inject
    let injected_module = syringe.inject(dll_dst)?;
    println!("injected");

    // check version
    let remote_version = unsafe {
        syringe.get_payload_procedure::<fn() -> String>(injected_module, "hook_version")
    }?
    .context(format!("{:?} has no 'hook_version' export", DLL_NAME))?;
    let hook_version = remote_version.call()?;
    let injector_version = env!("CARGO_PKG_VERSION").to_string();

    if injector_version != hook_version {
        bail!(
            "Mismatched version, injector is {:?} hook is {:?}",
            injector_version,
            hook_version
        )
    }

    // set log folder
    let remote_init = unsafe {
        syringe.get_payload_procedure::<fn(String) -> Result<(), String>>(injected_module, "init")
    }?
    .context(format!("{:?} has no 'init' export", DLL_NAME))?;
    if let Err(e) = remote_init.call(&log_dir.to_string())? {
        bail!(e);
    }
    println!("init called");

    Ok(())
}
