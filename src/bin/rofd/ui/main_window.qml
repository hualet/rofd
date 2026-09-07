import QtQuick 2.15
import QtQuick.Window 2.15
import QtQuick.Dialogs

Window {
    id: root
    visible: true
    width: 960
    height: 720
    title: ofdViewer.page_count > 0
           ? "rofd - " + (ofdViewer.current_page + 1) + " / " + ofdViewer.page_count
           : "rofd"

    FileDialog {
        id: openDialog
        title: "Open OFD file"
        nameFilters: ["OFD documents (*.ofd)", "All files (*)"]
        onAccepted: ofdViewer.open_file(selectedFile.toString())
    }

    component ToolButton: Rectangle {
        property alias text: label.text
        signal clicked

        width: label.implicitWidth + 24
        height: 32
        radius: 4
        color: mouseArea.containsMouse ? "#d8d8d8" : "#ececec"
        border.color: "#999999"

        Text {
            id: label
            anchors.centerIn: parent
        }

        MouseArea {
            id: mouseArea
            anchors.fill: parent
            hoverEnabled: true
            onClicked: parent.clicked()
        }
    }

    Column {
        anchors.fill: parent

        Row {
            id: toolbar
            spacing: 8
            padding: 8

            ToolButton {
                text: "Open…"
                onClicked: openDialog.open()
            }

            ToolButton {
                text: "‹"
                visible: ofdViewer.page_count > 0
                onClicked: ofdViewer.previous_page()
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: ofdViewer.page_count > 0
                text: (ofdViewer.current_page + 1) + " / " + ofdViewer.page_count
            }

            ToolButton {
                text: "›"
                visible: ofdViewer.page_count > 0
                onClicked: ofdViewer.next_page()
            }
        }

        Image {
            width: parent.width
            height: parent.height - toolbar.height
            source: ofdViewer.page_source
            fillMode: Image.PreserveAspectFit
            cache: false
        }
    }
}
