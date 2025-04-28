# utils.py
import os
import sys
import subprocess
import shutil
from PyQt6 import QtWidgets

# Required external tools
REQUIRED_TOOLS = [
    "dd", "wipefs", "parted", "mkfs.vfat", "mkfs.ntfs",
    "rsync", "mount", "umount"
]

# Mapping tools to packages per distribution
PKG_MAP = {
    'arch': {
        'dd': ['coreutils'],
        'wipefs': ['util-linux'],
        'parted': ['parted'],
        'mkfs.vfat': ['dosfstools'],
        'mkfs.ntfs': ['ntfs-3g'],
        'rsync': ['rsync'],
        'mount': ['util-linux'],
        'umount': ['util-linux']
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
        'umount': ['util-linux']
    }
}

def detect_distribution() -> str:
    """Detect Linux distribution from /etc/os-release."""
    try:
        info = {}
        with open('/etc/os-release') as f:
            for line in f:
                if '=' in line:
                    key, val = line.rstrip().split('=', 1)
                    info[key] = val.strip('"')
        return info.get('ID', '').lower()
    except FileNotFoundError:
        return ''


def prompt_install_packages(distro: str, packages: list[str]):
    """Prompt or instruct user to install missing packages."""
    msg = 'Missing packages: ' + ', '.join(packages)
    if distro == 'arch':
        resp = QtWidgets.QMessageBox.question(
            None,
            'Install Dependencies',
            f"{msg}\nInstall via pacman -Sy?",
            QtWidgets.QMessageBox.StandardButton.Yes |
            QtWidgets.QMessageBox.StandardButton.No
        )
        if resp == QtWidgets.QMessageBox.StandardButton.Yes:
            subprocess.run(['pkexec', 'pacman', '-Sy'] + packages)
    elif distro in ('debian', 'ubuntu'):
        QtWidgets.QMessageBox.information(
            None,
            'Install Dependencies',
            f"{msg}\nRun: sudo apt update && sudo apt install {' '.join(packages)}"
        )
    elif distro == 'fedora':
        QtWidgets.QMessageBox.information(
            None,
            'Install Dependencies',
            f"{msg}\nRun: sudo dnf install {' '.join(packages)}"
        )
    else:
        QtWidgets.QMessageBox.warning(
            None,
            'Missing Dependencies',
            msg
        )


def check_and_install_dependencies():
    """Check for required tools and prompt installation if missing."""
    missing = [tool for tool in REQUIRED_TOOLS if shutil.which(tool) is None]
    if not missing:
        return
    distro = detect_distribution()
    pkgmap = PKG_MAP.get(distro, {})
    packages = []
    for tool in missing:
        packages += pkgmap.get(tool, [])
    unique_pkgs = sorted(set(packages)) or missing
    prompt_install_packages(distro, unique_pkgs)


def ensure_root_via_pkexec():
    """Re-launch the script via pkexec to obtain root permissions."""
    if os.geteuid() != 0:
        script = os.path.realpath(sys.argv[0])
        env = [
            f"DISPLAY={os.environ.get('DISPLAY', ':0')}",
            f"XAUTHORITY={os.environ.get('XAUTHORITY', os.path.expanduser('~/.Xauthority'))}",
            f"HOME={os.environ.get('ORIG_HOME', os.environ.get('HOME', ''))}",
            f"XDG_CURRENT_DESKTOP={os.environ.get('XDG_CURRENT_DESKTOP', '')}"
        ]
        os.execvp(
            'pkexec',
            ['pkexec', 'env'] + env + [sys.executable, script] + sys.argv[1:]
        )
