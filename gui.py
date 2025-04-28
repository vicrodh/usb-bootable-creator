import os
import json
import hashlib
import subprocess
from PyQt6 import QtWidgets
from worker import WorkerThread

class MainWindow(QtWidgets.QMainWindow):
    """GUI for USB Bootable Creator with checksum, cluster size,
    cancel, dual-progress bars with proper signal connections."""
    def __init__(self):
        super().__init__()
        self.setWindowTitle('USB Bootable Creator')
        self.setMinimumWidth(650)
        self._setup_ui()

    def _setup_ui(self):
        central = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(central)

        # ISO selection
        iso_layout = QtWidgets.QHBoxLayout()
        iso_layout.addWidget(QtWidgets.QLabel('ISO File:'))
        self.iso_edit = QtWidgets.QLineEdit()
        iso_layout.addWidget(self.iso_edit)
        btn_browse = QtWidgets.QPushButton('Browse')
        btn_browse.clicked.connect(self._browse_iso)
        iso_layout.addWidget(btn_browse)
        layout.addLayout(iso_layout)

        # Checksum collapse
        self.chk_verify = QtWidgets.QCheckBox('Verify SHA256 checksum')
        layout.addWidget(self.chk_verify)
        self.verify_widget = QtWidgets.QWidget()
        verify_layout = QtWidgets.QHBoxLayout(self.verify_widget)
        verify_layout.addWidget(QtWidgets.QLabel('Expected SHA256:'))
        self.hash_edit = QtWidgets.QLineEdit()
        verify_layout.addWidget(self.hash_edit)
        btn_verify = QtWidgets.QPushButton('Verify')
        btn_verify.clicked.connect(self._verify_checksum)
        verify_layout.addWidget(btn_verify)
        self.note_label = QtWidgets.QLabel('')
        verify_layout.addWidget(self.note_label)
        self.verify_widget.setVisible(False)
        layout.addWidget(self.verify_widget)
        self.chk_verify.toggled.connect(self.verify_widget.setVisible)

        # Cluster size selection
        cs_layout = QtWidgets.QHBoxLayout()
        cs_layout.addWidget(QtWidgets.QLabel('Cluster size:'))
        self.cs_combo = QtWidgets.QComboBox()
        for size in ['512K', '1M', '2M', '4M', '8M', '16M', '32M', '64M']:
            self.cs_combo.addItem(size)
        self.cs_combo.setCurrentText('4M')
        cs_layout.addWidget(self.cs_combo)
        layout.addLayout(cs_layout)

        # Device selection
        dev_layout = QtWidgets.QHBoxLayout()
        dev_layout.addWidget(QtWidgets.QLabel('USB Device:'))
        self.dev_combo = QtWidgets.QComboBox()
        dev_layout.addWidget(self.dev_combo)
        btn_refresh = QtWidgets.QPushButton('Refresh')
        btn_refresh.clicked.connect(self._load_devices)
        dev_layout.addWidget(btn_refresh)
        layout.addLayout(dev_layout)

        # WIM option
        self.wim_chk = QtWidgets.QCheckBox('Use wimlib to split install.wim')
        layout.addWidget(self.wim_chk)

        # Progress bars
        self.overall_bar = QtWidgets.QProgressBar()
        self.overall_bar.setRange(0, 100)
        layout.addWidget(self.overall_bar)
        self.step_bar = QtWidgets.QProgressBar()
        self.step_bar.setRange(0, 100)
        layout.addWidget(self.step_bar)

        # Log area
        self.log_area = QtWidgets.QPlainTextEdit()
        self.log_area.setReadOnly(True)
        layout.addWidget(self.log_area)

        # Start and Cancel buttons
        btn_layout = QtWidgets.QHBoxLayout()
        self.btn_start = QtWidgets.QPushButton('Start')
        self.btn_start.clicked.connect(self._start_process)
        btn_layout.addWidget(self.btn_start)
        self.btn_cancel = QtWidgets.QPushButton('Cancel')
        self.btn_cancel.clicked.connect(self._cancel_confirm)
        btn_layout.addWidget(self.btn_cancel)
        layout.addLayout(btn_layout)

        self.setCentralWidget(central)
        self._load_devices()

    def _browse_iso(self):
        path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self, 'Select ISO', os.environ.get('ORIG_HOME', os.environ.get('HOME', '')),
            'ISO Files (*.iso)'
        )
        if path:
            self.iso_edit.setText(path)

    def _verify_checksum(self):
        iso = self.iso_edit.text().strip()
        exp = self.hash_edit.text().strip().lower()
        if not os.path.isfile(iso) or not exp:
            return
        self.log_area.appendPlainText('Computing SHA256 checksum...')
        sha256 = hashlib.sha256()
        with open(iso, 'rb') as f:
            for chunk in iter(lambda: f.read(65536), b''):
                sha256.update(chunk)
        if sha256.hexdigest() == exp:
            self.log_area.appendPlainText('Checksum valid.')
            self.note_label.setText('')
            self.btn_start.setEnabled(True)
        else:
            self.log_area.appendPlainText('Checksum mismatch!')
            self.note_label.setText('Invalid checksum')
            self.btn_start.setEnabled(False)

    def _load_devices(self):
        self.dev_combo.clear()
        try:
            out = subprocess.check_output([
                'lsblk', '-J', '-o', 'NAME,RM,SIZE,MODEL,TRAN,TYPE'
            ], text=True)
            for d in json.loads(out).get('blockdevices', []):
                if d.get('type') == 'disk' and (
                   d.get('rm') or str(d.get('rm')) == '1' or
                   (d.get('tran') or '').lower() == 'usb'
                ):
                    path = f"/dev/{d['name']}"
                    label = f"{path} - {d['size']} ({d.get('model','').strip()})"
                    self.dev_combo.addItem(label, path)
        except Exception as e:
            self.log_area.appendPlainText(f'Device error: {e}')

    def _cancel_confirm(self):
        resp = QtWidgets.QMessageBox.question(
            self, 'Cancel', 'Cleanup and exit?',
            QtWidgets.QMessageBox.StandardButton.Yes |
            QtWidgets.QMessageBox.StandardButton.No
        )
        if resp == QtWidgets.QMessageBox.StandardButton.Yes:
            self._cancel_process()
            self.close()

    def _cancel_process(self):
        for m in ('/mnt/ntfs', '/mnt/vfat', '/mnt/iso'):
            if os.path.ismount(m):
                subprocess.run(['umount', m], check=False)
            try:
                os.rmdir(m)
            except:
                pass
        self.log_area.appendPlainText('Cleanup complete.')
        self.btn_start.setEnabled(True)

    def _start_process(self):
        if self.chk_verify.isChecked() and not self.btn_start.isEnabled():
            QtWidgets.QMessageBox.warning(
                self, 'Checksum', 'Please verify checksum first.'
            )
            return
        iso = self.iso_edit.text().strip()
        dev = self.dev_combo.currentData()
        if not iso or not os.path.isfile(iso):
            QtWidgets.QMessageBox.warning(
                self, 'Invalid ISO', 'Select a valid ISO.'
            )
            return
        if not dev:
            QtWidgets.QMessageBox.warning(
                self, 'No Device', 'Select a USB device.'
            )
            return
        resp = QtWidgets.QMessageBox.question(
            self, 'Confirm', f'All data on {dev} will be erased. Continue?',
            QtWidgets.QMessageBox.StandardButton.Yes |
            QtWidgets.QMessageBox.StandardButton.No
        )
        if resp != QtWidgets.QMessageBox.StandardButton.Yes:
            return
        self.btn_start.setEnabled(False)
        self.log_area.clear()
        cluster = self.cs_combo.currentText()
        self.worker = WorkerThread(iso, dev, self.wim_chk.isChecked(), cluster)
        self.worker.overall.connect(lambda v: self.overall_bar.setValue(v))
        self.worker.step.connect(lambda v: self.step_bar.setValue(v))
        self.worker.log.connect(self.log_area.appendPlainText)
        self.worker.done.connect(lambda ok, msg: self._on_done(ok, msg))
        self.worker.start()

    def _on_done(self, success, message):
        if success:
            QtWidgets.QMessageBox.information(
                self, 'Success', 'USB creation completed.'
            )
        else:
            QtWidgets.QMessageBox.critical(
                self, 'Error', f'Failed: {message}'
            )
        self.btn_start.setEnabled(True)
