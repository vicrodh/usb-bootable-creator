import QtQuick 2.15
import QtQuick.Controls 2.15

ApplicationWindow {
    id: mainWindow
    width: 650
    height: 500
    visible: true
    title: "USB Bootable Creator"

    property var bridge: null

    Component.onCompleted: {
        if (bridge) {
            bridge.refresh_usb_devices();
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        // Spinner and OS Glyph indicator
        Row {
            spacing: 12
            // OS Glyph (placeholder)
            Label {
                id: osIcon
                text: "💻"
                font.pixelSize: 32
                width: 32; height: 32
            }
            // Spinner indicator
            BusyIndicator {
                id: spinner
                running: false
                width: 32; height: 32
            }
        }

        // ISO selection
        Row {
            spacing: 8
            Label { text: "ISO File:" }
            TextField { id: isoEdit; width: 300; text: bridge ? bridge.iso_path : "" }
            Button { text: "Browse"; onClicked: bridge && bridge.browse_iso() }
        }

        // Checksum collapse
        CheckBox {
            id: chkVerify
            text: "Verify SHA256 checksum"
        }
        Row {
            visible: chkVerify.checked
            spacing: 8
            Label { text: "Expected SHA256:" }
            TextField { id: hashEdit; width: 250 }
            Button { text: "Verify"; onClicked: {} }
            Label { id: noteLabel; text: "" }
        }

        // Cluster size selection
        Row {
            spacing: 8
            Label { text: "Cluster size:" }
            ComboBox {
                id: csCombo
                model: ["512K", "1M", "2M", "4M", "8M", "16M", "32M", "64M"]
                currentIndex: 3
            }
        }

        // Device selection
        Row {
            spacing: 8
            Label { text: "USB Device:" }
            ComboBox {
                id: devCombo
                width: 200
                model: bridge ? bridge.usb_devices : []
                textRole: "label"
                valueRole: "dev"
            }
            Button { text: "Refresh"; onClicked: bridge && bridge.refresh_usb_devices() }
        }

        // WIM option
        CheckBox {
            id: wimChk
            text: "Use wimlib to split install.wim"
        }

        // Log area
        TextArea {
            id: logArea
            readOnly: true
            height: 120
            width: parent.width
            wrapMode: TextArea.Wrap
        }

        // Start and Cancel buttons
        Row {
            spacing: 8
            Button { text: "Start"; onClicked: {} }
            Button { text: "Cancel"; onClicked: {} }
        }
    }
}
