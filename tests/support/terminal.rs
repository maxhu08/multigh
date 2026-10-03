use std::{
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

struct Session {
    child: Child,
    master: File,
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }

            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn terminal(command: &mut Command, steps: &[(&str, &[u8])]) -> (ExitStatus, String) {
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
        ws_row: 30,
        ws_col: 120,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            )
        },
        0
    );

    let master = unsafe { File::from_raw_fd(master) };
    let slave = unsafe { File::from_raw_fd(slave) };

    for fd in [master.as_raw_fd(), slave.as_raw_fd()] {
        assert_ne!(
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            -1
        );
    }
    command
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()));
    if !command.get_envs().any(|(key, _)| key == "TERM") {
        command.env("TERM", "xterm-256color");
    }

    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut session = Session {
        child: command.spawn().unwrap(),
        master,
    };

    drop(slave);

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut output = Vec::new();
    let mut step = 0;
    let mut search = 0;
    let mut status = None;
    let mut closed = false;

    while Instant::now() < deadline {
        let mut poll = libc::pollfd {
            fd: session.master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = if closed {
            std::thread::sleep(Duration::from_millis(10));
            0
        } else {
            unsafe { libc::poll(&mut poll, 1, 50) }
        };

        if ready > 0 {
            let mut buffer = [0; 8192];

            match session.master.read(&mut buffer) {
                Ok(0) => closed = true,
                Ok(count) => {
                    let chunk = &buffer[..count];

                    if chunk.windows(4).any(|window| window == b"\x1b[6n") {
                        session.master.write_all(b"\x1b[1;1R").unwrap();
                    }

                    output.extend_from_slice(chunk);

                    if let Some((needle, keys)) = steps.get(step)
                        && String::from_utf8_lossy(&output[search..]).contains(needle)
                    {
                        session.master.write_all(keys).unwrap();
                        search = output.len();
                        step += 1;
                    }
                }
                Err(error) if error.raw_os_error() == Some(libc::EIO) => closed = true,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => panic!("Read test terminal: {error}"),
            }
        }
        status = session.child.try_wait().unwrap();
        if status.is_some() && ready == 0 {
            break;
        }
    }
    let output = String::from_utf8_lossy(&output).into_owned();
    let status = status
        .or_else(|| session.child.try_wait().unwrap())
        .unwrap_or_else(|| panic!("Test terminal timed out: {output}"));

    assert_eq!(
        step,
        steps.len(),
        "Terminal did not reach expected prompts: {output}"
    );
    (status, output)
}
