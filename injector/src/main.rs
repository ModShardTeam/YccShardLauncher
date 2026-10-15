use std::fs;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::process::Command;

use clap::Parser;
use dll_syringe::{Syringe, process::OwnedProcess};
use windows::Win32::Foundation::{HANDLE, WAIT_FAILED, WAIT_TIMEOUT};
use windows::Win32::System::Threading::WaitForInputIdle;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    game: PathBuf,
}

fn main() {
    // TODO: remove unwraps, excepts and use results
    let cli = Cli::parse();

    let game_path = fs::canonicalize(cli.game).unwrap();
    let parent = game_path.parent().unwrap();

    if !game_path.is_file() {
        // TODO: error handling
        // TODO: check if it's a correct .exe file with maybe a .win in the same folder
        // TODO: add maybe some more checks
        panic!("Not a valid exe {:#?}", game_path);
    }

    // copy dll into the game folder
    // TODO: not hardcoded
    let dll = parent.join("hook.dll");
    fs::copy("target/debug/hook.dll", &dll).unwrap();

    // launch game
    let Ok(child) = Command::new(&game_path)
        .current_dir(parent)
        .spawn()
    else {
        // TODO: error handling
        panic!("Cannot launch {:#?}", game_path);
    };

    // get process
    let process: OwnedProcess = OwnedProcess::from_pid(child.id()).expect("game is not running");
    let syringe = Syringe::for_process(process);

    // wait for process to be initialized
    let wait = unsafe { WaitForInputIdle(HANDLE(child.as_raw_handle() as _), 30_000) };
    if wait == WAIT_FAILED.0 {
        panic!("wait failed");
    } else if wait == WAIT_TIMEOUT.0 {
        println!("wait stopped because of timeout");
    } else if wait == 0 {
        println!("wait succeeded");
    }

    // inject
    syringe.inject(dll).expect("injection failed");
    println!("injected");
}
