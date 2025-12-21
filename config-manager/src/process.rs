use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to send sighup: {0}")]
    Sighup(#[from] nix::Error),
}

pub type Pid = i32;

#[derive(Debug)]
pub struct Process {
    pub pid: Pid,
    pub cmdline: Option<String>,
}

impl Process {
    pub fn sighup(&self) -> Result<(), Error> {
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.pid),
            nix::sys::signal::Signal::SIGHUP,
        )
        .map_err(Error::Sighup)
    }
}

pub struct ProcessIter {
    pids: Vec<Pid>,
    pos: usize,
}

impl ProcessIter {
    pub fn try_new() -> io::Result<Self> {
        let pids = fs::read_dir("/proc")?.try_fold(vec![], |mut acc, entry| {
            let entry = entry?;
            let Ok(pid) = entry.file_name().to_string_lossy().parse::<Pid>() else {
                return Ok(acc);
            };

            acc.push(pid);
            Ok::<_, io::Error>(acc)
        })?;

        Ok(Self { pids, pos: 0 })
    }
}

impl Iterator for ProcessIter {
    type Item = Process;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(pid) = self.pids.get(self.pos).copied() else {
            return None;
        };

        self.pos += 1;

        fn get_cmdline(pid: Pid) -> Option<String> {
            let path = PathBuf::from("/proc").join(pid.to_string()).join("cmdline");
            fs::read_to_string(path).map(|v| v.trim().to_string()).ok()
        }

        let cmdline = get_cmdline(pid);
        Some(Process { pid, cmdline })
    }
}
