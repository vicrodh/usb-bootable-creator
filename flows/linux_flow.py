import os
import select
import subprocess
import tempfile
import shutil


def linux_dd_flow(iso: str, device: str, cluster: str, signals):
    """Perform dd copy of a Linux ISO with progress updates using a random mount base."""
    # Create random directory base for future mounts or temp use
    base = tempfile.mkdtemp(prefix='usbcreator_', dir='/mnt')
    try:
        # Initial verbose log
        signals.log.emit('Starting dd copy...')

        # Build dd command
        cmd = [
            'dd',
            f'if={iso}',
            f'of={device}',
            f'bs={cluster}',
            'conv=fdatasync',
            'status=progress'
        ]
        proc = subprocess.Popen(cmd, stderr=subprocess.PIPE)

        # Track total size for progress
        total = os.path.getsize(iso)
        buf = ''
        fd = proc.stderr.fileno()

        # Read and log progress from dd stderr
        while proc.poll() is None:
            r, _, _ = select.select([fd], [], [], 0.5)
            if fd in r:
                chunk = os.read(fd, 1024).decode(errors='ignore')
                buf += chunk
                # Parse carriage-returned progress lines
                while '\r' in buf:
                    line, buf = buf.split('\r', 1)
                    if 'bytes' in line:
                        try:
                            num = int(line.split(' bytes')[0])
                        except ValueError:
                            continue
                        pct = min(int(num * 100 / total), 100)
                        signals.log.emit(f'Copied {pct}%')

        proc.wait()
        if proc.returncode != 0:
            raise RuntimeError('dd failed')

        signals.log.emit('Syncing to disk...')
        subprocess.run(['sync'], check=False)
        signals.log.emit('dd copy completed.')
    finally:
        # Cleanup temporary mount base
        shutil.rmtree(base, ignore_errors=True)
