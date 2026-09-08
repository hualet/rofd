import QtQuick
import QtQuick.Controls.Basic
ToolButton {
    id: control
    property string hint: text
    property string symbol: ""
    implicitWidth: symbol ? 34 : Math.max(34, contentItem.implicitWidth + 20)
    implicitHeight: 34
    hoverEnabled: true
    font.pixelSize: 14
    ToolTip.visible: hovered
    ToolTip.delay: 600
    ToolTip.text: hint
    Accessible.name: hint
    contentItem: Text {
        text: control.symbol ? "" : control.text
        ReaderIcon { anchors.centerIn: parent; name: control.symbol; visible: name.length > 0; ink: !control.enabled ? "#b5bdc9" : control.checked ? "#2463cf" : "#354156" }
        font: control.font
        color: !control.enabled ? "#b5bdc9" : control.checked ? "#2463cf" : "#354156"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        radius: 6
        color: control.down ? "#dce7fb" : control.checked ? "#eaf1ff" : control.hovered ? "#edf0f5" : "transparent"
        border.color: control.activeFocus ? "#3675df" : "transparent"
    }
}
