//! Unix-only subprocess tests use an isolated PTY, never the user's terminal.
use std::fs::File;
use std::io::{self, Read};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

pub struct Pty {
    pub master: File,
    slave: File,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Mode {
    pub flags: [libc::tcflag_t; 4],
    control_chars: Vec<libc::cc_t>,
}

pub struct PtyChild(Child);
impl Drop for PtyChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn contains(output: &[u8], needle: &[u8]) -> bool {
    output.windows(needle.len()).any(|window| window == needle)
}

impl Pty {
    pub fn new() -> Self {
        let (mut master, mut slave) = (-1, -1);
        let size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: both descriptor pointers and the size live for the syscall;
        // null name/termios pointers request defaults. Successful descriptors
        // are transferred once to File, which closes them on drop.
        let result = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                &size,
            )
        };
        assert_eq!(result, 0, "openpty: {}", io::Error::last_os_error());
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        // SAFETY: the live master descriptor supports fcntl; only its status
        // flags change, allowing reads to remain bounded by the test deadline.
        let flags = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
            0
        );
        Self { master, slave }
    }

    pub fn mode(&self) -> Mode {
        let mut attributes = MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr initializes the provided termios on success, and
        // the slave descriptor remains live for this entire PTY session.
        assert_eq!(
            unsafe { libc::tcgetattr(self.slave.as_raw_fd(), attributes.as_mut_ptr()) },
            0
        );
        let attributes = unsafe { attributes.assume_init() };
        Mode {
            flags: [
                attributes.c_iflag,
                attributes.c_oflag,
                attributes.c_cflag,
                attributes.c_lflag,
            ],
            control_chars: attributes.c_cc.to_vec(),
        }
    }

    pub fn spawn(&self, command: &mut Command) -> PtyChild {
        PtyChild(
            command
                .stdin(self.slave.try_clone().unwrap())
                .stdout(self.slave.try_clone().unwrap())
                .stderr(self.slave.try_clone().unwrap())
                .env("TERM", "xterm-256color")
                .spawn()
                .unwrap(),
        )
    }

    fn drain(&mut self, output: &mut Vec<u8>) {
        let mut buffer = [0; 4096];
        loop {
            match self.master.read(&mut buffer) {
                Ok(0) => return,
                Ok(count) => output.extend_from_slice(&buffer[..count]),
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock
                        || error.raw_os_error() == Some(libc::EIO) =>
                {
                    return;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => panic!("PTY read: {error}"),
            }
        }
    }

    pub fn wait_for(&mut self, child: &mut PtyChild, output: &mut Vec<u8>, needle: &[u8]) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.drain(output);
            if contains(output, needle) {
                return;
            }
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "child exited before marker: {}",
                String::from_utf8_lossy(output)
            );
            assert!(
                Instant::now() < deadline,
                "PTY marker timeout: {}",
                String::from_utf8_lossy(output)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn wait(&mut self, child: &mut PtyChild, output: &mut Vec<u8>) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.drain(output);
            if let Some(status) = child.0.try_wait().unwrap() {
                self.drain(output);
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "PTY exit timeout: {}",
                String::from_utf8_lossy(output)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
