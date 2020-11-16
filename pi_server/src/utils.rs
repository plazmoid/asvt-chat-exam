use nix::{
    sys::signal::{signal, SigHandler, Signal},
    unistd::{close as fdclose, fork, getppid, setsid, ForkResult},
};
use std::process::exit;

pub fn daemonize() -> Result<i32, String> {
    if getppid().as_raw() != 1 {
        setsig(Signal::SIGTTOU, SigHandler::SigIgn);
        setsig(Signal::SIGTTIN, SigHandler::SigIgn);
        setsig(Signal::SIGTSTP, SigHandler::SigIgn);
    }
    for fd in 0..=2 {
        match fdclose(fd) {
            _ => (),
        }
    }

    unsafe {
        match fork() {
            Ok(ForkResult::Parent { .. }) => {
                exit(0);
            }
            Ok(ForkResult::Child) => match setsid() {
                Ok(pid) => Ok(pid.as_raw()),
                Err(e) => Err(e.to_string()),
            },
            Err(_) => exit(255),
        }
    }
}

pub fn setsig(sig: Signal, hnd: SigHandler) {
    unsafe {
        signal(sig, hnd).unwrap();
    }
}
