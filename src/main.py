#!/usr/bin/env python3
"""
main.py — Graphical USB Bootable Creator for Linux with PolicyKit elevation,
real-time progress updates, Windows ISO detection,
user environment integration, and distribution-specific dependency management.
"""
import os
import sys
import subprocess
import json
import select
import shutil
from PyQt6 import QtWidgets, QtCore

# Preserve original user environment variables
ORIG_HOME = os.environ.get("ORIG_HOME", os.environ.get("HOME", ""))
XDG_CURRENT_DESKTOP = os.environ.get("XDG_CURRENT_DESKTOP", "")
QT_THEME = os.environ.get("QT_QPA_PLATFORMTHEME", "")

# Required external tools
REQUIRED_TOOLS = [
    "dd", "wipefs", "parted", "mkfs.vfat", "mkfs.ntfs",
    "rsync", "mount", "umount"
]

# Distribution-specific package mappings
PKG_MAP = {
    'arch': {
        'dd': ['coreutils'],
        'wipefs': ['util-linux'],
        'parted': ['parted'],
        'mkfs.vfat': ['dosfstools'],
        'mkfs.ntfs': ['ntfs-3g'],
        'rsync': ['rsync'],
        'mount': ['util-linux'],
        'umount': ['util-linux'],
    },
    'debian': {},
    'ubuntu': {},
    'fedora': {
        'dd': ['coreutils'],
        'wipefs': ['util-linux'],
        'parted': ['parted'],
        'mkfs.vfat': ['dosfstools'],
        'mkfs.ntfs': ['ntfs-3g'],
        'rsync': ['rsync'],
        'mount': ['util-linux'],
        'umount': ['util-linux'],
    },
}


def detect_distribution() -> str:
    """
    Return the Linux distribution ID by reading /etc/os-release.
    """
    try:
        info = {}
        with open('/etc/os-release') as f:
            for line in f:
                if '=' in line:
                    key, val = line.rstrip().split('=', 1)
                    info[key] = val.strip('"')
        return info.get('ID', '').lower()
    except Exception:
        return ''


def prompt_install_packages(distro: str, packages: list[str]):
    """
    Prompt user or instruct to install missing packages.
    """
    if distro == 'arch':
        resp = QtWidgets.QMessageBox.question(
            None,
            'Install Dependencies (Arch)',
            'Missing packages: ' + ', '.join(packages) + '\nInstall via pacman -Sy? ',
            QtWidgets.QMessageBox.StandardButton.Yes |
            QtWidgets.QMessageBox.StandardButton.No
        )
        if resp == QtWidgets.QMessageBox.StandardButton.Yes:
            subprocess.run(['pkexec', 'pacman', '-Sy'] + packages)
    elif distro in ('debian', 'ubuntu'):
        QtWidgets.QMessageBox.information(
            None,
            'Install Dependencies',
            'Run:\n sudo apt update && sudo apt install ' + ' '.join(packages)
        )
    elif distro == 'fedora':
        QtWidgets.QMessageBox.information(
            None,
            'Install Dependencies',
            'Run:\n sudo dnf install ' + ' '.join(packages)
        )
    else:
        QtWidgets.QMessageBox.warning(
            None,
            'Missing Dependencies',
            'Please install: ' + ', '.join(packages)
        )


def check_and_install_dependencies():
    """
    Check for required tools and prompt installation if missing.
    """
    missing = [t for t in REQUIRED_TOOLS if shutil.which(t) is None]
    if not missing:
        return
    distro = detect_distribution()
    pkgmap = PKG_MAP.get(distro, {})
    pkgs = []
    for tool in missing:
        pkgs += pkgmap.get(tool, [])
    pkgs = sorted(set(pkgs))
    if pkgs:
        prompt_install_packages(distro, pkgs)
    else:
        prompt_install_packages('other', missing)


def ensure_root_via_pkexec():
    """
    Relaunch script via pkexec if not root, preserving environment.
    """
    if os.geteuid() != 0:
        script = os.path.realpath(sys.argv[0])
        env = [
            f'DISPLAY={os.environ.get("DISPLAY",":0")}',
            f'XAUTHORITY={os.environ.get("XAUTHORITY",os.path.expanduser("~/.Xauthority"))}',
            f'ORIG_HOME={ORIG_HOME}',
            f'XDG_CURRENT_DESKTOP={XDG_CURRENT_DESKTOP}'
        ]
        if QT_THEME:
            env.append(f'QT_QPA_PLATFORMTHEME={QT_THEME}')
        os.execvp(
            'pkexec', ['pkexec', 'env'] + env + [sys.executable, script] + sys.argv[1:]
        )


class WorkerThread(QtCore.QThread):
    """Thread to perform USB creation tasks."""
    progress = QtCore.pyqtSignal(int)
    log = QtCore.pyqtSignal(str)
    done = QtCore.pyqtSignal(bool, str)

    def __init__(self, iso: str, device: str, use_wim: bool):
        super().__init__()
        self.iso = iso
        self.device = device
        self.use_wim = use_wim

    def run(self):
        try:
            if self._is_windows_iso():
                self._run_windows_flow()
            else:
                self._run_dd_flow()
            self.done.emit(True, '')
        except FileNotFoundError as e:
            tool = os.path.basename(e.filename)
            self.log.emit(f'Required tool not found: {tool}')
            self.done.emit(False, f'Missing: {tool}')
        except Exception as e:
            self.log.emit(f'ERROR: {e}')
            self.done.emit(False, str(e))

    def _is_windows_iso(self) -> bool:
        mountpt = '/tmp/iso_detect'
        os.makedirs(mountpt, exist_ok=True)
        try:
            # Unmount any existing ISO mount
            with open('/proc/mounts') as mounts:
                for line in mounts:
                    if self.iso in line:
                        mp = line.split()[1]
                        subprocess.run(['umount', mp], check=False)
                        self.log.emit(f'Unmounted existing ISO at {mp}')
                        break
            subprocess.run(['mount', '-o', 'loop,ro', self.iso, mountpt], check=True)
            return os.path.isfile(f'{mountpt}/bootmgr') and os.path.isdir(f'{mountpt}/sources')
        finally:
            subprocess.run(['umount', mountpt], check=False)
            os.rmdir(mountpt)

    def _run_dd_flow(self):
        self.log.emit('Starting dd copy...')
        cmd = [
            'dd',
            f'if={self.iso}',
            f'of={self.device}',
            'bs=4M',
            'conv=fdatasync',
            'status=progress'
        ]
        proc = subprocess.Popen(cmd, stderr=subprocess.PIPE)
        total = os.path.getsize(self.iso)
        buf = ''
        fd = proc.stderr.fileno()
        last_pct = -1
        while proc.poll() is None:
            r, _, _ = select.select([fd], [], [], 0.5)
            if fd in r:
                chunk = os.read(fd, 1024).decode(errors='ignore')
                buf += chunk
                while '\r' in buf:
                    line, buf = buf.split('\r', 1)
                    if 'bytes' in line:
                        try:
                            num = int(line.split(' bytes')[0])
                        except ValueError:
                            continue
                        pct = min(int(num * 100 / total), 100)
                        self.progress.emit(pct)
                        if last_pct >= 0 and pct != last_pct:
                            self.log.emit(f'Copied {pct}%')
                        last_pct = pct
        proc.wait()
        if proc.returncode != 0:
            raise RuntimeError('dd failed')
        self.log.emit('Syncing to disk...')
        subprocess.run(['sync'], check=False)
        self.progress.emit(100)
        self.log.emit('dd copy completed.')

    def _run_windows_flow(self):
        self.log.emit('Running Windows USB creation...')
        # Step 1: wipe and partition
        self.log.emit('Wiping existing signatures...')
        subprocess.run(['wipefs', '-a', self.device], check=True)
        self.progress.emit(5)
        self.log.emit('Creating GPT partition table...')
        subprocess.run(['parted', '-s', self.device, 'mklabel', 'gpt'], check=True)
        self.progress.emit(10)
        self.log.emit('Creating FAT32 BOOT partition...')
        subprocess.run([
            'parted', '-s', self.device,
            'mkpart', 'BOOT', 'fat32', '0%', '1GiB'
        ], check=True)
        self.progress.emit(15)
        self.log.emit('Creating NTFS INSTALL partition...')
        subprocess.run([
            'parted', '-s', self.device,
            'mkpart', 'INSTALL', 'ntfs', '1GiB', '100%'
        ], check=True)
        self.progress.emit(20)
        QtCore.QThread.sleep(1)
        p1 = f'{self.device}1'
        p2 = f'{self.device}2'
        # Step 2: format
        self.log.emit('Formatting BOOT as FAT32...')
        subprocess.run(['mkfs.vfat', '-F32', '-n', 'BOOT', p1], check=True)
        self.progress.emit(25)
        self.log.emit('Formatting INSTALL as NTFS...')
        subprocess.run(['mkfs.ntfs', '--quick', '-L', 'INSTALL', p2], check=True)
        self.progress.emit(30)
        # Mount dirs
        iso_m = '/mnt/iso'
        vfat_m = '/mnt/vfat'
        ntfs_m = '/mnt/ntfs'
        for m in (iso_m, vfat_m, ntfs_m):
            os.makedirs(m, exist_ok=True)
        # Step 3: mount ISO
        self.log.emit('Mounting ISO...')
        res = subprocess.run([
            'mount', '-o', 'loop,ro', self.iso, iso_m
        ], capture_output=True, text=True)
        if res.returncode != 0 and 'already mounted' not in res.stderr:
            raise RuntimeError(f"Failed to mount ISO: {res.stderr.strip()}")
        self.progress.emit(35)
        # Step 4: BOOT
        self.log.emit('Mounting BOOT partition...')
        subprocess.run(['mount', p1, vfat_m], check=True)
        self.log.emit('Copying to BOOT (excluding sources)...')
        subprocess.run([
            'rsync', '-a', '--no-owner', '--no-group',
            '--exclude', 'sources/', f'{iso_m}/', f'{vfat_m}/'
        ], check=True)
        self.progress.emit(60)
        self.log.emit('Copying boot.wim to BOOT...')
        os.makedirs(f'{vfat_m}/sources', exist_ok=True)
        subprocess.run(['cp', f'{iso_m}/sources/boot.wim', f'{vfat_m}/sources'], check=True)
        self.progress.emit(70)
        # Step 5: INSTALL
        self.log.emit('Mounting INSTALL partition...')
        subprocess.run(['mount', p2, ntfs_m], check=True)
        self.log.emit('Copying to INSTALL...')
        subprocess.run([
            'rsync', '-a', '--no-owner', '--no-group', f'{iso_m}/', f'{ntfs_m}/'
        ], check=True)
        self.progress.emit(90)
        # Step 6: cleanup
        self.log.emit('Cleaning up mounts...')
        for m in (ntfs_m, vfat_m, iso_m):
            subprocess.run(['umount', m], check=False)
        for m in (ntfs_m, vfat_m, iso_m):
            try:
                os.rmdir(m)
            except OSError:
                pass
        subprocess.run(['sync'], check=False)
        self.progress.emit(100)
        self.log.emit('Windows USB creation completed.')


class MainWindow(QtWidgets.QMainWindow):
    """GUI for USB Bootable Creator."""
    def __init__(self):
        super().__init__()
        self.setWindowTitle('USB Bootable Creator')
        self.setMinimumWidth(600)
        self._setup_ui()

    def _setup_ui(self):
        central = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(central)

        # ISO selection
        row1 = QtWidgets.QHBoxLayout()
        row1.addWidget(QtWidgets.QLabel('ISO File:'))
        self.iso_edit = QtWidgets.QLineEdit()
        btn_iso = QtWidgets.QPushButton('Browse')
        btn_iso.clicked.connect(self._browse_iso)
        row1.addWidget(self.iso_edit)
        row1.addWidget(btn_iso)
        layout.addLayout(row1)

        # Device selection
        row2 = QtWidgets.QHBoxLayout()
        row2.addWidget(QtWidgets.QLabel('USB Device:'))
        self.dev_combo = QtWidgets.QComboBox()
        btn_refresh = QtWidgets.QPushButton('Refresh')
        btn_refresh.clicked.connect(self._load_devices)
        row2.addWidget(self.dev_combo)
        row2.addWidget(btn_refresh)
        layout.addLayout(row2)

        # WIM option
        self.wim_chk = QtWidgets.QCheckBox('Use wimlib to split install.wim')
        layout.addWidget(self.wim_chk)

        # Progress and log
        self.pbar = QtWidgets.QProgressBar()
        self.pbar.setRange(0, 100)
        layout.addWidget(self.pbar)
        self.log_area = QtWidgets.QPlainTextEdit()
        self.log_area.setReadOnly(True)
        layout.addWidget(self.log_area)

        # Start button
        btn_start = QtWidgets.QPushButton('Start')
        btn_start.clicked.connect(self._start_process)
        layout.addWidget(btn_start)

        self.setCentralWidget(central)
        self._load_devices()

    def _browse_iso(self):
        start_dir = ORIG_HOME
        path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self,
            'Select ISO',
            start_dir,
            'ISO Files (*.iso)'
        )
        if path:
            self.iso_edit.setText(path)

    def _load_devices(self):
        self.dev_combo.clear()
        try:
            out = subprocess.check_output([
                'lsblk', '-J', '-o', 'NAME,RM,SIZE,MODEL,TRAN,TYPE'
            ], text=True)
            devs = json.loads(out).get('blockdevices', [])
            for d in devs:
                if d.get('type') == 'disk' and (
                        d.get('rm') in [True, '1', 1] or
                        (d.get('tran') or '').lower() == 'usb'
                ):
                    path = f"/dev/{d['name']}"
                    label = f"{path} - {d['size']}"
                    if d.get('model'):
                        label += f" ({d['model'].strip()})"
                    self.dev_combo.addItem(label, path)
        except Exception as e:
            self.log_area.appendPlainText(f'Device error: {e}')

    def _start_process(self):
        iso = self.iso_edit.text().strip()
        dev = self.dev_combo.currentData()
        if not iso or not os.path.isfile(iso):
            QtWidgets.QMessageBox.warning(
                self,
                'Invalid ISO',
                'Please select a valid ISO.'
            )
            return
        if not dev:
            QtWidgets.QMessageBox.warning(
                self,
                'No Device',
                'Please select a USB device.'
            )
            return
        resp = QtWidgets.QMessageBox.question(
            self,
            'Confirm',
            f'All data on {dev} will be erased. Continue?',
            QtWidgets.QMessageBox.StandardButton.Yes |
            QtWidgets.QMessageBox.StandardButton.No
        )
        if resp != QtWidgets.QMessageBox.StandardButton.Yes:
            return

        self.iso_edit.setEnabled(False)
        self.dev_combo.setEnabled(False)
        self.wim_chk.setEnabled(False)
        self.pbar.setValue(0)
        self.log_area.clear()

        self.worker = WorkerThread(iso, dev, self.wim_chk.isChecked())
        self.worker.progress.connect(self.pbar.setValue)
        self.worker.log.connect(self.log_area.appendPlainText)
        self.worker.done.connect(self._on_done)
        self.worker.start()

    def _on_done(self, success: bool, msg: str):
        self.iso_edit.setEnabled(True)
        self.dev_combo.setEnabled(True)
        self.wim_chk.setEnabled(True)
        if success:
            QtWidgets.QMessageBox.information(
                self, 'Success', 'USB creation completed successfully.'
            )
        else:
            QtWidgets.QMessageBox.critical(
                self, 'Error', f'Failed: {msg}'
            )


if __name__ == '__main__':
    app = QtWidgets.QApplication(sys.argv)
    # Apply user theme: breeze for KDE/Plasma, Fusion otherwise
    theme = (
        'breeze' if 'plasma' in XDG_CURRENT_DESKTOP.lower()
        or 'kde' in XDG_CURRENT_DESKTOP.lower() else 'Fusion'
    )
    app.setStyle(QtWidgets.QStyleFactory.create(theme))
    check_and_install_dependencies()
    ensure_root_via_pkexec()
    win = MainWindow()
    win.show()
    sys.exit(app.exec())
