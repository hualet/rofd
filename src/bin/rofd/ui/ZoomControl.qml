import QtQuick
import QtQuick.Controls.Basic
Rectangle {
    id: control
    property real value: 1
    signal zoomRequested(real value)
    implicitWidth: 90
    implicitHeight: 32
    radius: 6
    color: "#f5f7fa"
    border.color: input.activeFocus ? "#3675df" : "#dfe4ec"
    function restoreText() { input.text = Qt.binding(function() { return Math.round(control.value * 100) + "%" }) }
    TextField {
        id: input
        objectName: "zoomInput"
        Accessible.name: "缩放百分比"
        width: parent.width - 26; height: parent.height
        padding: 5
        horizontalAlignment: Text.AlignHCenter
        text: Math.round(control.value * 100) + "%"
        selectByMouse: true
        background: null
        onEditingFinished: {
            let typed = text.trim()
            if (/^\d+(\.\d+)?%?$/.test(typed)) control.zoomRequested(parseFloat(typed) / 100)
            control.restoreText()
        }
    }
    ReaderButton {
        anchors.right: parent.right
        width: 26; height: parent.height
        symbol: "down"; hint: "选择缩放比例"
        onClicked: presets.popup()
        Menu {
            id: presets
            Repeater {
                model: [25, 50, 75, 100, 125, 150, 200, 300, 400]
                MenuItem {
                    required property int modelData
                    text: modelData + "%"
                    onTriggered: control.zoomRequested(modelData / 100)
                }
            }
        }
    }
}
