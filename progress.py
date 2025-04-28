# progress.py
from PyQt6 import QtCore

class ProgressSignals(QtCore.QObject):
    """
    Signals for reporting progress and logging:
    - overall(int): overall percentage of the full task
    - step(int): percentage of the current sub-step
    - log(str): textual log messages
    - done(bool, str): completion status and optional error message
    """
    overall = QtCore.pyqtSignal(int)
    step    = QtCore.pyqtSignal(int)
    log     = QtCore.pyqtSignal(str)
    done    = QtCore.pyqtSignal(bool, str)
