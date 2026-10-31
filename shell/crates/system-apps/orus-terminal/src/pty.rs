use std::io;
use std::os::unix::io::{FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct Pty {
    master: RawFd,
    child: Child,
}

impl Pty {
    pub fn spawn(cols: u16, rows: u16, cwd: &Path) -> io::Result<Self> {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;

        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let ret = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &ws as *const libc::winsize as *mut libc::winsize,
            )
        };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }

        // Set master to non-blocking
        unsafe {
            let flags = libc::fcntl(master, libc::F_GETFL);
            libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        // Dup slave for stdout/stderr (from_raw_fd takes ownership)
        let slave_out = unsafe { libc::dup(slave) };
        let slave_err = unsafe { libc::dup(slave) };

        let child = unsafe {
            Command::new(&shell)
                .arg("-l")
                .current_dir(cwd)
                .stdin(Stdio::from_raw_fd(slave))
                .stdout(Stdio::from_raw_fd(slave_out))
                .stderr(Stdio::from_raw_fd(slave_err))
                .env("TERM", "xterm-256color")
                .pre_exec(|| {
                    libc::setsid();
                    libc::ioctl(0, libc::TIOCSCTTY as libc::c_ulong, 0);
                    Ok(())
                })
                .spawn()?
        };

        Ok(Self { master, child })
    }

    pub fn try_read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe {
            libc::read(
                self.master,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        if n < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    pub fn write_all(&self, data: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let n = unsafe {
                libc::write(
                    self.master,
                    data[offset..].as_ptr() as *const libc::c_void,
                    data.len() - offset,
                )
            };
            if n < 0 {
                return Err(io::Error::last_os_error());
            }
            offset += n as usize;
        }
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(self.master, libc::TIOCSWINSZ as libc::c_ulong, &ws) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
        // Signal the child's process group
        unsafe {
            libc::kill(-(self.child.id() as libc::pid_t), libc::SIGWINCH);
        }
        Ok(())
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.master);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
