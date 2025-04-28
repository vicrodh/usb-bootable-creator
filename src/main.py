#!/usr/bin/env python3
"""
main.py — Graphical USB Bootable Creator for Linux with PolicyKit elevation.
"""
import os
import sys
import subprocess
import json


def ensure_root_via_pkexec():
    """
    Relaunch via pkexec if not root, preserving virtualenv and X environment.
    """
    if os.geteuid() != 0:
        script = os.path.abspath(sys.argv[0])
        display = os.environ.get("DISPLAY", ":0")
        xauth = os.environ.get("XAUTHORITY", os.path.expanduser("~/.Xauthority"))
        cmd = [
            "pkexec",
            "env",
            f"DISPLAY={display}",
            f"XAUTHORITY={xauth}",
            sys.executable,
            script,
        ] + sys.argv[1:]
        os.execvp("pkexec", cmd)

# Ensure we have root privileges before PyQt imports
ensure_root_via_pkexec()

from PyQt6 import QtWidgets, QtCore  # noqa: E402


class WorkerThread(QtCore.QThread):
    """
    Thread to perform dd or wimlib operations without blocking UI.
    """
    progress = QtCore.pyqtSignal(int)
    log = QtCore.pyqtSignal(str)
    done = QtCore.pyqtSignal(bool, str)

    def __init__(self, iso, device, wim):
        super().__init__()
        self.iso = iso
        self.device = device
        self.wim = wim

    def run(self):
        try:
            if not self.wim:
                self._run_dd()
            else:
                self._run_wimlib()
            self.done.emit(True, "")
        except Exception as e:
            self.log.emit(f"ERROR: {e}")
            self.done.emit(False, str(e))

    def _run_dd(self):
        self.log.emit("Starting dd copy...")
        cmd = ["dd", f"if={self.iso}", f"of={self.device}", "bs=4M", "conv=fdatasync", "status=progress"]
        proc = subprocess.Popen(cmd, stderr=subprocess.PIPE, stdout=subprocess.DEVNULL)
        total = os.path.getsize(self.iso)
        while True:
            line = proc.stderr.readline()
            if not line:
                break
            text = line.decode(errors="ignore").strip()
            if "bytes" in text:
                val = text.split(" bytes")[0]
                try:
                    copied = int(val)
                except ValueError:
                    continue
                pct = int(copied * 100 / total) if total else 0
                self.progress.emit(min(pct, 100))
        if proc.wait() != 0:
            raise RuntimeError("dd failed")
        subprocess.run(["sync"])
        self.progress.emit(100)
        self.log.emit("dd completed.")

    def _run_wimlib(self):
        self.log.emit("Unmounting partitions...")
        mounts = []
        with open("/proc/mounts") as m:
            for l in m:
                if l.startswith(self.device):
                    mounts.append(l.split()[1])
        for mnt in mounts:
            subprocess.run(["umount", "-l", mnt], check=False)
        self.log.emit("Formatting drive and copying files...")
        subprocess.run(["parted", "-s", self.device, "mklabel", "msdos"], check=True)
        subprocess.run([
            "parted", "-s", self.device, "mkpart", "primary", "fat32", "1MiB", "100%"
        ], check=True)
        subprocess.run(["parted", "-s", self.device, "set", "1", "boot", "on"], check=True)
        QtCore.QThread.sleep(1)
        part = f"{self.device}1"
        subprocess.run(["mkfs.vfat", "-F32", "-n", "WINUSB", part], check=True)
        iso_m = "/tmp/iso_mount"
        usb_m = "/tmp/usb_mount"
        os.makedirs(iso_m, exist_ok=True)
        os.makedirs(usb_m, exist_ok=True)
        subprocess.run(["mount", "-o", "loop,ro", self.iso, iso_m], check=True)
        subprocess.run(["mount", part, usb_m], check=True)
        self._copy_files(iso_m, usb_m)
        subprocess.run(["sync"], check=False)
        subprocess.run(["umount", iso_m], check=False)
        subprocess.run(["umount", usb_m], check=False)
        for d in (iso_m, usb_m):
            try:
                os.rmdir(d)
            except OSError:
                pass
        self.progress.emit(100)
        self.log.emit("wimlib process completed.")

    def _copy_files(self, src, dest):
        split = False
        wimfile = None
        size_total = 0
        for r, _, files in os.walk(src):
            for f in files:
                path = os.path.join(r, f)
                if f.lower() == "install.wim" and os.path.basename(r).lower() == "sources":
                    if os.path.getsize(path) > 4 * 1024**3:
                        split = True
                        wimfile = path
                        continue
                size_total += os.path.getsize(path)
        copied = 0
        for r, _, files in os.walk(src):
            rel = os.path.relpath(r, src)
            base = dest if rel == "." else os.path.join(dest, rel)
            os.makedirs(base, exist_ok=True)
            for f in files:
                spath = os.path.join(r, f)
                if split and f.lower() == "install.wim" and os.path.basename(r).lower() == "sources":
                    self.log.emit("Skipping install.wim for splitting")
                    continue
                dpath = os.path.join(base, f)
                with open(spath, "rb") as sf, open(dpath, "wb") as df:
                    while chunk := sf.read(4 * 1024**2):
                        df.write(chunk)
                        copied += len(chunk)
                        pct = int(copied * 100 / size_total) if size_total else 100
                        self.progress.emit(min(pct, 100))
        self.log.emit("File copy done.")
        if split and wimfile:
            self.log.emit("Splitting WIM file...")
            out = os.path.join(dest, "sources", "install.swm")
            res = subprocess.run(["wimsplit", wimfile, out, "4000"], capture_output=True, text=True)
            if res.returncode != 0:
                raise RuntimeError(f"WIM split error: {res.stderr.strip()}")
            self.log.emit("WIM split done.")


class MainWindow(QtWidgets.QMainWindow):
    """
    Main window for USB Creator.
    """
    def __init__(self):
        super().__init__()
        self.setWindowTitle("USB Bootable Creator")
        self.setMinimumWidth(500)
        self._setup_ui()
        QtWidgets.QApplication.setStyle(QtWidgets.QStyleFactory.create("Fusion"))

    def _setup_ui(self):
        central = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(central)

        # ISO picker
        row = QtWidgets.QHBoxLayout()
        row.addWidget(QtWidgets.QLabel("ISO File:"))
        self.iso_edit = QtWidgets.QLineEdit()
        btn1 = QtWidgets.QPushButton("Browse")
        btn1.clicked.connect(self._browse_iso)
        row.addWidget(self.iso_edit)
        row.addWidget(btn1)
        layout.addLayout(row)

        # Device combo
        row2 = QtWidgets.QHBoxLayout()
        row2.addWidget(QtWidgets.QLabel("USB Device:"))
        self.dev_combo = QtWidgets.QComboBox()
        btn2 = QtWidgets.QPushButton("Refresh")
        btn2.clicked.connect(self._load_devices)
        row2.addWidget(self.dev_combo)
        row2.addWidget(btn2)
        layout.addLayout(row2)

        # wim option
        self.wim_chk = QtWidgets.QCheckBox("Use wimlib split for large install.wim")
        layout.addWidget(self.wim_chk)

        # Progress and log
        self.pbar = QtWidgets.QProgressBar()
        layout.addWidget(self.pbar)
        self.log = QtWidgets.QPlainTextEdit()
        self.log.setReadOnly(True)
        layout.addWidget(self.log)

        # Start
        st = QtWidgets.QPushButton("Start")
        st.clicked.connect(self._start)
        layout.addWidget(st)

        self.setCentralWidget(central)
        self._load_devices()

    def _browse_iso(self):
        file, _ = QtWidgets.QFileDialog.getOpenFileName(self, "Select ISO", "", "ISO Files (*.iso)")
        if file:
            self.iso_edit.setText(file)

    def _load_devices(self):
        """Detect and list removable USB disks."""
        self.dev_combo.clear()
        try:
            out = subprocess.run([
                "lsblk", "-J", "-o", "NAME,RM,SIZE,MODEL,TRAN,TYPE"
            ], capture_output=True, text=True, check=True)
            data = json.loads(out.stdout)
            for dev in data.get("blockdevices", []):
                if dev.get("type") == "disk" and (
                    dev.get("rm") in [True, "1", 1]
                    or (dev.get("tran") or "").lower() == "usb"
                ):
                    name = dev.get("name")
                    size = dev.get("size")
                    model = (dev.get("model") or "").strip()
                    path = f"/dev/{name}"
                    label = f"{path} - {size}" + (f" - {model}" if model else "")
                    self.dev_combo.addItem(label, path)
        except Exception as e:
            self.log.appendPlainText(f"Device listing error: {e}")

    def _start(self):
        iso = self.iso_edit.text().strip()
        dev = self.dev_combo.currentData()
        if not iso or not os.path.isfile(iso):
            QtWidgets.QMessageBox.warning(self, "Invalid ISO", "Select a valid ISO.")
            return
        if not dev:
            QtWidgets.QMessageBox.warning(self, "No Device", "Select a USB device.")
            return
        resp = QtWidgets.QMessageBox.question(
            self, "Confirm",
            f"All data on {dev} will be lost. Continue?",
            QtWidgets.QMessageBox.StandardButton.Yes | QtWidgets.QMessageBox.StandardButton.No
        )
        if resp != QtWidgets.QMessageBox.StandardButton.Yes:
            return

        # disable UI
        self.iso_edit.setEnabled(False)
        self.dev_combo.setEnabled(False)
        self.wim_chk.setEnabled(False)

        self.pbar.setValue(0)
        self.log.clear()

        self.worker = WorkerThread(iso, dev, self.wim_chk.isChecked())
        self.worker.progress.connect(self.pbar.setValue)
        self.worker.log.connect(self.log.appendPlainText)
        self.worker.done.connect(self._finish)
        self.worker.start()

    def _finish(self, ok, msg):
        self.iso_edit.setEnabled(True)
        self.dev_combo.setEnabled(True)
        self.wim_chk.setEnabled(True)
        if ok:
            QtWidgets.QMessageBox.information(self, "Done", "USB creation done.")
        else:
            QtWidgets.QMessageBox.critical(self, "Error", f"Failed: {msg}")


def main():
    app = QtWidgets.QApplication(sys.argv)
    win = MainWindow()
    win.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
