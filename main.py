#!/usr/bin/env python3
import sys
import os
from PyQt6 import QtWidgets
from utils import check_and_install_dependencies, ensure_root_via_pkexec
from gui import MainWindow

if __name__ == '__main__':
    # Create application
    app = QtWidgets.QApplication(sys.argv)
    # Apply user theme: breeze on KDE/Plasma, Fusion otherwise
    desktop = os.environ.get('XDG_CURRENT_DESKTOP', '').lower()
    theme = 'breeze' if 'plasma' in desktop or 'kde' in desktop else 'Fusion'
    app.setStyle(QtWidgets.QStyleFactory.create(theme))

    # Check and install dependencies if needed
    check_and_install_dependencies()
    # Relaunch as root via pkexec if not already
    ensure_root_via_pkexec()

    # Launch main window
    win = MainWindow()
    win.show()
    sys.exit(app.exec())
