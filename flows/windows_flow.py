import os
import subprocess
import tempfile
import shutil


def windows_flow(iso: str, device: str, use_wim: bool, signals):
    """Run Windows ISO partition, format, and copy with progress updates."""
    # Generate unique mount base
    base = tempfile.mkdtemp(prefix='usbcreator_', dir='/mnt')
    iso_m = os.path.join(base, 'iso')
    boot_m = os.path.join(base, 'boot')
    inst_m = os.path.join(base, 'install')
    for m in (iso_m, boot_m, inst_m):
        os.makedirs(m, exist_ok=True)

    try:
        # Stage 1: wipe and partition
        signals.log.emit('Wiping and partitioning...')
        subprocess.run(['wipefs', '-a', device], check=True)
        subprocess.run(['parted', '-s', device, 'mklabel', 'gpt'], check=True)
        signals.overall.emit(10)
        signals.step.emit(100)

        # Create partitions
        start = '0%'
        parts = [
            ('BOOT', 'fat32', '1GiB', 'BOOT'),
            ('ESD-USB', 'ntfs', '100%', 'ESD-USB')
        ]
        for label, fstype, end, vol in parts:
            signals.log.emit(f'Creating partition {label}...')
            subprocess.run([
                'parted', '-s', device,
                'mkpart', label, fstype, start, end
            ], check=True)
            start = end

        # Format partitions
        p1 = f"{device}1"
        p2 = f"{device}2"
        signals.log.emit('Formatting BOOT as FAT32...')
        subprocess.run(['mkfs.vfat', '-F32', '-n', 'BOOT', p1], check=True)
        signals.log.emit('Formatting INSTALL as NTFS...')
        subprocess.run(['mkfs.ntfs', '--quick', '-L', 'ESD-USB', p2], check=True)
        signals.overall.emit(30)
        signals.step.emit(100)

        # Mount ISO
        signals.log.emit('Mounting ISO...')
        subprocess.run(['mount', '-o', 'loop,ro', iso, iso_m], check=True)
        signals.overall.emit(35)
        signals.step.emit(100)

        # Copy BOOT files
        signals.log.emit('Mounting BOOT partition...')
        subprocess.run(['mount', p1, boot_m], check=True)
        signals.log.emit('Copying files to BOOT...')
        subprocess.run([
            'rsync', '-a', '--no-owner', '--no-group',
            '--exclude', 'sources/', f'{iso_m}/', f'{boot_m}/'
        ], check=True)
        signals.log.emit('Copying boot.wim...')
        os.makedirs(f'{boot_m}/sources', exist_ok=True)
        subprocess.run([
            'cp', f'{iso_m}/sources/boot.wim', f'{boot_m}/sources'
        ], check=True)
        signals.overall.emit(60)
        signals.step.emit(100)

        # Copy INSTALL files
        signals.log.emit('Mounting INSTALL partition...')
        subprocess.run(['mount', p2, inst_m], check=True)
        signals.log.emit('Copying files to INSTALL...')
        subprocess.run([
            'rsync', '-a', '--no-owner', '--no-group', f'{iso_m}/', f'{inst_m}/'
        ], check=True)
        signals.overall.emit(90)
        signals.step.emit(100)

    finally:
        # Cleanup mounts
        signals.log.emit('Cleaning up mounts...')
        for m in (inst_m, boot_m, iso_m):
            subprocess.run(['umount', m], check=False)
        # Remove mount dirs
        shutil.rmtree(base, ignore_errors=True)
        subprocess.run(['sync'], check=False)
        signals.overall.emit(100)
        signals.step.emit(100)
        signals.log.emit('Windows USB creation completed.')
