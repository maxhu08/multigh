import errno
import fcntl
import json
import os
import pty
import select
import signal
import struct
import sys
import termios
import time

steps = json.loads(os.environ['MGH_PTY_STEPS'])
(pid, fd) = pty.fork()
if pid == 0:
    os.execvp(sys.argv[1], sys.argv[1:])

fcntl.ioctl(
    fd,
    termios.TIOCSWINSZ,
    struct.pack('HHHH', 32, int(os.getenv('MGH_PTY_COLUMNS', '120')), 0, 0),
)
reaped = None
output = b''
cursor = 0
deadline = time.monotonic() + 25

try:
    for (needle, keys) in steps:
        while needle.encode() not in output[cursor:]:
            if time.monotonic() > deadline:
                raise RuntimeError('timeout waiting for ' + repr(needle))
            if select.select([fd], [], [], 0.1)[0]:
                try:
                    chunk = os.read(fd, 65536)
                except OSError as e:
                    if e.errno == errno.EIO:
                        raise RuntimeError('child exited waiting for ' + repr(needle))
                    raise
                if not chunk:
                    raise RuntimeError('EOF waiting for ' + repr(needle))
                output += chunk
        cursor = len(output)
        if '\x03' in keys:
            # Wait for raw input so Ctrl-C is read as a form key, rather than SIGINT.
            while termios.tcgetattr(fd)[3] & termios.ISIG:
                if time.monotonic() > deadline:
                    raise RuntimeError('prompt never entered raw input mode')
                time.sleep(0.005)
        os.write(fd, keys.encode())
        time.sleep(0.12)

    while time.monotonic() < deadline:
        if select.select([fd], [], [], 0.1)[0]:
            try:
                chunk = os.read(fd, 65536)
            except OSError as e:
                if e.errno == errno.EIO:
                    break
                raise
            if not chunk:
                break
            output += chunk
        (ended, status) = os.waitpid(pid, os.WNOHANG)
        if ended:
            reaped = status
            pid = 0
            break
    else:
        raise RuntimeError('child did not exit')

    if pid:
        (_, status) = os.waitpid(pid, 0)
        code = os.waitstatus_to_exitcode(status)
    else:
        code = os.waitstatus_to_exitcode(reaped)
    sys.stdout.buffer.write(output)
    sys.exit(code if code >= 0 else 1)
except Exception as e:
    sys.stdout.buffer.write(output)
    print(str(e), file=sys.stderr)
    if pid:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
    sys.exit(99)
