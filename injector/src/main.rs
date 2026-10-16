use std::ffi::c_void;
use std::fs;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode};

use anstream::{eprintln, println};
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

struct ChildGuard {
    child: Child,
    disarmed: bool,
}

impl ChildGuard {
    fn id(&self) -> u32 {
        self.child.id()
    }

    fn as_raw_handle(&self) -> *mut c_void {
        self.child.as_raw_handle()
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.disarmed {
            let _ = self.child.kill();
        }
    }
}

impl From<Child> for ChildGuard {
    fn from(value: Child) -> Self {
        ChildGuard {
            child: value,
            disarmed: false,
        }
    }
}

const DLL_NAME: &str = "hook.dll";
const ERR_STYLE: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)));
const MSG_STYLE: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
const WARN_STYLE: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));

fn run() -> Result<()> {
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
    println!(
        "{MSG_STYLE}Copying from {:?} to {:?}{MSG_STYLE:#}",
        dll_src, dll_dst
    );
    fs::copy(dll_src, &dll_dst)?;

    // launch game
    let mut child = ChildGuard::from(Command::new(&game_path).current_dir(parent).spawn()?);

    // get process
    let process: OwnedProcess = OwnedProcess::from_pid(child.id())?;
    let syringe = Syringe::for_process(process);

    // wait for process to be initialized
    let wait = unsafe { WaitForInputIdle(HANDLE(child.as_raw_handle() as _), 30_000) };
    if wait == WAIT_FAILED.0 {
        bail!("Wait failed");
    } else if wait == WAIT_TIMEOUT.0 {
        println!(
            "{WARN_STYLE}Game is still waiting for input idle, injecting may fail.{WARN_STYLE:#}"
        );
    } else if wait == 0 {
        println!("{MSG_STYLE}Game is correctly initialized{MSG_STYLE:#}");
    }

    // inject
    let injected_module = syringe.inject(dll_dst)?;
    println!("{MSG_STYLE}DLL Injection succeeded{MSG_STYLE:#}");

    // TODO: do the following check before launching the game
    // TODO: but I cant rely on syringe and needs some C-ABI
    // TODO: maybe later
    // check version and git hash
    let remote_info = unsafe {
        syringe
            .get_payload_procedure::<fn() -> (String, Option<String>)>(injected_module, "hook_info")
    }?
    .with_context(|| format!("{} has no 'hook_info' export", DLL_NAME))?;
    let (hook_version, hook_hash) = remote_info.call()?;
    let injector_version = env!("CARGO_PKG_VERSION").to_string();
    let injector_hash = option_env!("GIT_HASH").map(str::to_string);

    if injector_version != hook_version {
        bail!(
            "Mismatched version, injector is {:?} hook is {:?}",
            injector_version,
            hook_version
        )
    }

    if injector_hash != hook_hash {
        bail!(
            "Mismatched git hash, injector is {} hook is {}",
            injector_hash.as_deref().unwrap_or("dev build"),
            hook_hash.as_deref().unwrap_or("dev build")
        )
    } else if injector_hash.is_none() {
        println!("{WARN_STYLE}dev build: hash check skipped{WARN_STYLE:#}");
    }

    // set log folder
    let remote_init = unsafe {
        syringe.get_payload_procedure::<fn(String) -> Result<(), String>>(injected_module, "init")
    }?
    .with_context(|| format!("{} has no 'init' export", DLL_NAME))?;
    if let Err(e) = remote_init.call(&log_dir.to_string())? {
        bail!(e);
    }
    println!("{MSG_STYLE}DLL Init called with success{MSG_STYLE:#}");

    child.disarm();
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("{ERR_STYLE}Error: {e:#}{ERR_STYLE:#}");

        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
