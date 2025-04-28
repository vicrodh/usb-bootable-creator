import os
import select
import subprocess


def linux_dd_flow(iso: str, device: str, cluster: str, signals):
    """Perform dd copy of a Linux ISO with progress updates."""
    # Initial verbose log and overall start
    signals.log.emit('Starting dd copy...')
    signals.overall.emit(0)

    # Prepare dd command
    cmd = [
        'dd',
        f'if={iso}',
        f'of={device}',
        f'bs={cluster}',
        'conv=fdatasync',
        'status=progress'
    ]
    proc = subprocess.Popen(cmd, stderr=subprocess.PIPE)

    # Calculate total size for progress
    total = os.path.getsize(iso)
    buf = ''
    fd = proc.stderr.fileno()
    last_pct = -1

    # Read and parse dd progress
    while proc.poll() is None:
        r, _, _ = select.select([fd], [], [], 0.5)
        if fd in r:
            chunk = os.read(fd, 1024).decode(errors='ignore')
            buf += chunk
            # dd uses carriage returns to update the same line
            while '\r' in buf:
                line, buf = buf.split('\r', 1)
                if 'bytes' in line:
                    try:
                        num = int(line.split(' bytes')[0])
                    except ValueError:
                        continue
                    pct = min(int(num * 100 / total), 100)
                    signals.step.emit(pct)
                    if last_pct >= 0 and pct != last_pct:
                        signals.log.emit(f'Copied {pct}%')
                    last_pct = pct

    proc.wait()
    # Ensure step bar completes
    signals.step.emit(100)

    # Update overall to 90% before syncing
    signals.overall.emit(90)
    signals.log.emit('Syncing to disk...')
    subprocess.run(['sync'], check=False)

    # Finalize overall
    signals.overall.emit(100)
    signals.log.emit('dd copy completed.')
