# worker.py
import os
import subprocess
from PyQt6 import QtCore
from flows.linux_flow import linux_dd_flow
from flows.windows_flow import windows_flow

class WorkerThread(QtCore.QThread):
    """Thread to perform Linux or Windows ISO writing with progress signals."""
    # Define signals directly on the thread for correct Qt6 behavior
    overall = QtCore.pyqtSignal(int)
    step    = QtCore.pyqtSignal(int)
    log     = QtCore.pyqtSignal(str)
    done    = QtCore.pyqtSignal(bool, str)

    def __init__(self, iso: str, device: str, use_wim: bool, cluster: str):
        super().__init__()
        self.iso = iso
        self.device = device
        self.use_wim = use_wim
        self.cluster = cluster

    def run(self):
        try:
            if self._is_windows_iso():
                self.log.emit('Detected Windows ISO')
                windows_flow(self.iso, self.device, self.use_wim, self)
            else:
                self.log.emit('Detected Linux ISO')
                linux_dd_flow(self.iso, self.device, self.cluster, self)
            self.done.emit(True, '')
        except Exception as e:
            self.log.emit(f'ERROR: {e}')
            self.done.emit(False, str(e))

    def _is_windows_iso(self) -> bool:
        """Mount ISO briefly and check for Windows-specific files."""
        mountpt = '/tmp/iso_detect'
        os.makedirs(mountpt, exist_ok=True)
        try:
            subprocess.run(
                ['mount', '-o', 'loop,ro', self.iso, mountpt],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
            )
            return os.path.isfile(os.path.join(mountpt, 'bootmgr')) and \
                   os.path.isdir(os.path.join(mountpt, 'sources'))
        except subprocess.CalledProcessError:
            return False
        finally:
            subprocess.run(['umount', mountpt], check=False)
            try:
                os.rmdir(mountpt)
            except OSError:
                pass
