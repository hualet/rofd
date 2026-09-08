import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
Rectangle {
    id: pane
    required property var bridge
    property var pages: []
    property var results: []
    property int activeResult: -1
    property int tab: 0
    property string submittedQuery: ""
    signal jumpRequested(int page, var region)
    signal resultRequested(int index)
    signal nextRequested(int direction)
    color: "#f8f9fc"
    function focusSearch() { tab = 1; search.forceActiveFocus() }
    function reset() { debounce.stop(); search.text = ""; submittedQuery = "" }
    function submit() { submittedQuery = search.text; pane.bridge.search(search.text) }
    function enter(direction) {
        debounce.stop()
        if (submittedQuery !== search.text) submit()
        else pane.nextRequested(direction)
    }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        TabBar {
            Layout.fillWidth: true; currentIndex: pane.tab
            background: Rectangle { color: "#f8f9fc" }
            onCurrentIndexChanged: pane.tab = currentIndex
            TabButton {
                text: "页面"; implicitHeight: 42
                contentItem: Label { text: parent.text; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter; color: parent.checked ? "#2463cf" : "#798394" }
                background: Rectangle { color: "transparent"; Rectangle { anchors.bottom: parent.bottom; anchors.horizontalCenter: parent.horizontalCenter; width: 32; height: 2; radius: 1; color: "#3675df"; visible: parent.parent.checked } }
            }
            TabButton {
                text: "搜索"; implicitHeight: 42
                contentItem: Label { text: parent.text; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter; color: parent.checked ? "#2463cf" : "#798394" }
                background: Rectangle { color: "transparent"; Rectangle { anchors.bottom: parent.bottom; anchors.horizontalCenter: parent.horizontalCenter; width: 32; height: 2; radius: 1; color: "#3675df"; visible: parent.parent.checked } }
            }
        }
        ListView {
            id: thumbs
            visible: pane.tab === 0
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true; spacing: 16; topMargin: 18; bottomMargin: 18
            model: pane.pages
            currentIndex: pane.bridge.current_page
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            ScrollBar.vertical: ScrollBar { }
            delegate: Item {
                required property var modelData
                required property int index
                width: thumbs.width; height: preview.height + 27
                Rectangle {
                    anchors.centerIn: preview; width: preview.width + 6; height: preview.height + 6
                    color: "transparent"; radius: 3
                    border.width: index === pane.bridge.current_page ? 2 : 0; border.color: "#3675df"
                }
                Loader {
                    id: preview
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 134; height: Math.min(240, 134 * modelData.height / modelData.width)
                    active: parent.y + parent.height >= thumbs.contentY - thumbs.height / 2
                        && parent.y <= thumbs.contentY + thumbs.height * 1.5
                    sourceComponent: PageSurface {
                        bridge: pane.bridge; pageNumber: index; thumbnail: true
                        renderScale: 134 / modelData.width * pane.Screen.devicePixelRatio
                    }
                }
                Label {
                    anchors.top: preview.bottom; anchors.topMargin: 8; anchors.horizontalCenter: parent.horizontalCenter
                    text: index + 1; font.pixelSize: 12; color: index === pane.bridge.current_page ? "#2463cf" : "#798394"
                }
                MouseArea { anchors.fill: parent; onClicked: pane.jumpRequested(index, null) }
            }
        }
        ColumnLayout {
            visible: pane.tab === 1; Layout.fillWidth: true; Layout.fillHeight: true
            Layout.margins: 12; spacing: 8
            TextField {
                id: search
                background: Rectangle { radius: 6; color: "white"; border.color: search.activeFocus ? "#3675df" : "#dfe4ec" }
                Accessible.name: "搜索文档内容"
                Layout.fillWidth: true; placeholderText: "搜索文档内容…"; selectByMouse: true
                onTextEdited: debounce.restart()
                Keys.onReturnPressed: function(event) { pane.enter(event.modifiers & Qt.ShiftModifier ? -1 : 1) }
                Keys.onEnterPressed: function(event) { pane.enter(event.modifiers & Qt.ShiftModifier ? -1 : 1) }
                Timer { id: debounce; interval: 250; onTriggered: pane.submit() }
            }
            RowLayout {
                Layout.fillWidth: true
                Label { Layout.fillWidth: true; text: pane.results.length ? (pane.activeResult + 1) + " / " + pane.results.length + " 处匹配" : "全文搜索"; color: "#68758a"; font.pixelSize: 12 }
                ReaderButton { symbol: "up"; hint: "上一处（Shift+Enter）"; enabled: pane.results.length > 0; onClicked: pane.nextRequested(-1) }
                ReaderButton { symbol: "down"; hint: "下一处（Enter）"; enabled: pane.results.length > 0; onClicked: pane.nextRequested(1) }
            }
            Label { Layout.fillWidth: true; text: pane.bridge.search_status; visible: text.length > 0; wrapMode: Text.Wrap; color: "#798394"; font.pixelSize: 12 }
            BusyIndicator { Layout.alignment: Qt.AlignHCenter; running: pane.bridge.search_busy; visible: running; implicitHeight: 30 }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 6
                model: pane.results; currentIndex: pane.activeResult
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                ScrollBar.vertical: ScrollBar { }
                delegate: ItemDelegate {
                    required property var modelData
                    required property int index
                    width: ListView.view.width
                    implicitHeight: resultText.implicitHeight + 24
                    background: Rectangle { radius: 6; color: index === pane.activeResult ? "#e8effd" : "#ffffff" }
                    contentItem: Column {
                        id: resultText; spacing: 6
                        Label { text: "第 " + (modelData.page + 1) + " 页"; color: "#3675df"; font.pixelSize: 11 }
                        Label { width: parent.width; text: modelData.snippet; textFormat: Text.PlainText; wrapMode: Text.Wrap; maximumLineCount: 3; elide: Text.ElideRight; color: "#354156"; font.pixelSize: 12 }
                    }
                    onClicked: pane.resultRequested(index)
                }
            }
        }
    }
}
