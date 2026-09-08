import QtQuick
import QtQuick.Window
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Dialogs
import "ReaderMath.js" as MathUtil
ApplicationWindow {
    id: root
    visible: true
    width: 1120; height: 800; minimumWidth: 800; minimumHeight: 560
    color: "#edf0f5"
    title: ofdViewer.document_title ? ofdViewer.document_title + " — rofd" : "rofd 阅读器"
    font.family: "Noto Sans CJK SC"; font.pixelSize: 13
    property var pages: JSON.parse(ofdViewer.pages_json || "[]")
    property var results: JSON.parse(ofdViewer.search_json || "[]")
    property int activeResult: -1
    property bool sidebar: true
    property real zoom: 1
    property string fitMode: "width"
    property bool resetViewport: true
    function setZoom(value, mode) {
        let a = document.anchor()
        document.restoring = true
        fitMode = mode || "custom"
        zoom = MathUtil.clampZoom(value)
        document.restore(a)
        document.restoring = false
        document.syncCurrentPage()
    }
    function fit(mode) {
        if (!pages.length || document.width <= 48 || document.height <= 48) return
        let page = pages[Math.max(0, Math.min(ofdViewer.current_page, pages.length - 1))]
        let pageWidth = mode === "width" ? Math.max.apply(null, pages.map(function(p) { return p.width })) : page.width
        let value = (document.width - 48) / pageWidth
        if (mode === "page") value = Math.min(value, (document.height - 48) / page.height)
        setZoom(value, mode)
        if (resetViewport) { document.jump(0, null); resetViewport = false }
    }
    function navigate(index, region) {
        if (!pages.length) return
        index = Math.max(0, Math.min(index, pages.length - 1))
        if (fitMode === "page") {
            let page = pages[index]
            setZoom(Math.min((document.width - 48) / page.width, (document.height - 48) / page.height), "page")
            region = null
        }
        document.jump(index, region)
    }
    function showSearch() { sidebar = true; navigation.focusSearch() }
    function selectResult(index) {
        if (index < 0 || index >= results.length) return
        activeResult = index
        root.navigate(results[index].page, results[index])
    }
    function nextResult(direction) { selectResult(MathUtil.nextResult(activeResult, direction, results.length)) }
    onResultsChanged: { activeResult = -1; if (results.length) selectResult(0) }
    FileDialog { id: openDialog; title: "打开 OFD 文档"; nameFilters: ["OFD 文档 (*.ofd *.OFD)", "所有文件 (*)"]; onAccepted: ofdViewer.open_file(selectedFile.toString()) }
    Timer { interval: 30; repeat: true; running: true; onTriggered: ofdViewer.poll() }
    Connections {
        target: ofdViewer
        function onGeneration_changed() {
            navigation.reset(); activeResult = -1; resetViewport = true
            document.contentY = 0; document.contentX = 0
            Qt.callLater(function() { root.fit(root.fitMode === "custom" ? "width" : root.fitMode); document.jump(0, null) })
        }
    }
    header: Rectangle {
        height: 58; color: "#ffffff"
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: "#dde2ea" }
        RowLayout {
            anchors.fill: parent; anchors.margins: 11; spacing: 5
            ReaderButton { text: "打开"; hint: "打开文档（Ctrl+O）"; onClicked: openDialog.open() }
            Label { Layout.fillWidth: true; Layout.minimumWidth: 24; text: ofdViewer.document_title || "rofd"; textFormat: Text.PlainText; elide: Text.ElideMiddle; color: "#354156"; font.weight: Font.DemiBold }
            ReaderButton { symbol: "sidebar"; hint: "显示 / 隐藏侧栏"; checkable: true; checked: root.sidebar; onClicked: root.sidebar = !root.sidebar }
            Rectangle { width: 1; height: 22; color: "#e3e7ed"; Layout.leftMargin: 3; Layout.rightMargin: 3 }
            ReaderButton { symbol: "previous"; hint: "上一页"; enabled: ofdViewer.current_page > 0; onClicked: root.navigate(ofdViewer.current_page - 1, null) }
            TextField {
                id: pageInput; Accessible.name: "页码";
                background: Rectangle { radius: 6; color: "#f5f7fa"; border.color: pageInput.activeFocus ? "#3675df" : "#dfe4ec" } Layout.preferredWidth: 42; implicitHeight: 32
                text: pages.length ? ofdViewer.current_page + 1 : "0"
                horizontalAlignment: Text.AlignHCenter; selectByMouse: true; enabled: pages.length > 0
                onAccepted: {
                    let value = Number(text)
                    if (Number.isInteger(value) && value >= 1 && value <= pages.length) root.navigate(value - 1, null)
                    text = Qt.binding(function() { return pages.length ? ofdViewer.current_page + 1 : "0" })
                    focus = false
                }
            }
            Label { text: "/ " + pages.length; color: "#8490a1"; Layout.minimumWidth: 27 }
            ReaderButton { symbol: "next"; hint: "下一页"; enabled: ofdViewer.current_page + 1 < pages.length; onClicked: root.navigate(ofdViewer.current_page + 1, null) }
            Rectangle { width: 1; height: 22; color: "#e3e7ed"; Layout.leftMargin: 3; Layout.rightMargin: 3 }
            ReaderButton { symbol: "minus"; hint: "缩小（Ctrl+−）"; enabled: pages.length > 0 && root.zoom > 0.25; onClicked: root.setZoom(root.zoom / 1.1) }
            ZoomControl {
                Layout.preferredWidth: 90; enabled: pages.length > 0
                value: root.zoom
                onZoomRequested: function(value) { root.setZoom(value) }
            }
            ReaderButton { symbol: "plus"; hint: "放大（Ctrl++）"; enabled: pages.length > 0 && root.zoom < 4; onClicked: root.setZoom(root.zoom * 1.1) }
            ReaderButton { visible: root.width >= 960; text: "适宽"; hint: "适合宽度"; enabled: pages.length > 0; checked: root.fitMode === "width"; onClicked: root.fit("width") }
            ReaderButton { visible: root.width >= 960; text: "整页"; hint: "显示整页"; enabled: pages.length > 0; checked: root.fitMode === "page"; onClicked: root.fit("page") }
            ReaderButton { visible: root.width < 960; text: "适配"; enabled: pages.length > 0; onClicked: fitMenu.popup()
                Menu { id: fitMenu; MenuItem { text: "适合宽度"; onTriggered: root.fit("width") } MenuItem { text: "显示整页"; onTriggered: root.fit("page") } }
            }
            ReaderButton { text: "搜索"; hint: "搜索（Ctrl+F）"; enabled: pages.length > 0; onClicked: root.showSearch() }
        }
    }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            Layout.fillWidth: true; implicitHeight: errorRow.implicitHeight + 16; visible: ofdViewer.error_message.length > 0; color: "#fff0ed"
            RowLayout { id: errorRow; anchors.fill: parent; anchors.margins: 8
                Label { Layout.fillWidth: true; text: ofdViewer.error_message; textFormat: Text.PlainText; wrapMode: Text.Wrap; color: "#a44338" }
                ReaderButton { symbol: "close"; hint: "关闭提示"; onClicked: ofdViewer.clear_error() }
            }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 0
            NavigationPane {
                id: navigation; Layout.preferredWidth: 216; Layout.fillHeight: true
                visible: root.sidebar && pages.length > 0
                bridge: ofdViewer; pages: root.pages; results: root.results; activeResult: root.activeResult
                onJumpRequested: function(page, region) { root.navigate(page, region) }
                onResultRequested: function(index) { root.selectResult(index) }
                onNextRequested: function(direction) { root.nextResult(direction) }
            }
            Rectangle { Layout.fillHeight: true; width: 1; color: "#dde2ea"; visible: navigation.visible }
            DocumentView {
                id: document; Layout.fillWidth: true; Layout.fillHeight: true; visible: pages.length > 0
                bridge: ofdViewer; pages: root.pages; zoom: root.zoom; results: root.results; activeResult: root.activeResult
                onZoomRequested: function(value) { root.setZoom(value) }
                onWidthChanged: resizeFit.restart()
                onHeightChanged: resizeFit.restart()
                Timer { id: resizeFit; interval: 80; onTriggered: if (root.fitMode !== "custom") root.fit(root.fitMode) }
            }
            Item {
                Layout.fillWidth: true; Layout.fillHeight: true; visible: pages.length === 0
                Column {
                    anchors.centerIn: parent; spacing: 18
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter; width: 72; height: 88; radius: 8; color: "#ffffff"; border.color: "#d7dfeb"
                        Label { anchors.centerIn: parent; text: "OFD"; font.pixelSize: 21; font.weight: Font.DemiBold; color: "#3675df" }
                    }
                    Label { anchors.horizontalCenter: parent.horizontalCenter; text: "每一页，清晰呈现"; font.pixelSize: 23; color: "#354156" }
                    Label { anchors.horizontalCenter: parent.horizontalCenter; text: "打开 OFD 文档，开始阅读与搜索"; color: "#8490a1" }
                    Button { anchors.horizontalCenter: parent.horizontalCenter; text: "打开文档"; highlighted: true; onClicked: openDialog.open() }
                    Label { anchors.horizontalCenter: parent.horizontalCenter; text: "Ctrl + O"; font.pixelSize: 12; color: "#9aa4b3" }
                }
            }
        }
    }
    footer: Rectangle {
        height: 30; color: "#fafbfd"
        RowLayout { anchors.fill: parent; anchors.leftMargin: 16; anchors.rightMargin: 16
            Label { text: pages.length ? "第 " + (ofdViewer.current_page + 1) + " 页，共 " + pages.length + " 页" : "准备就绪"; font.pixelSize: 11; color: "#798394" }
            Item { Layout.fillWidth: true }
            Label { text: ofdViewer.busy ? "正在打开文档…" : ""; color: "#3675df"; font.pixelSize: 11 }
            Label { text: pages.length ? Math.round(root.zoom * 100) + "%" : ""; color: "#798394"; font.pixelSize: 11 }
        }
    }
    Shortcut { sequence: "Ctrl+O"; onActivated: openDialog.open() }
    Shortcut { sequence: "Ctrl+F"; enabled: pages.length > 0; onActivated: root.showSearch() }
    Shortcut { sequence: "Escape"; enabled: navigation.tab === 1; onActivated: { navigation.tab = 0; navigation.reset(); document.forceActiveFocus(); root.activeResult = -1; ofdViewer.search("") } }
    Shortcut { sequences: ["Ctrl++", "Ctrl+="]; onActivated: root.setZoom(root.zoom * 1.1) }
    Shortcut { sequence: "Ctrl+-"; onActivated: root.setZoom(root.zoom / 1.1) }
    Shortcut { sequence: "Ctrl+Home"; onActivated: root.navigate(0, null) }
    Shortcut { sequence: "Ctrl+End"; onActivated: root.navigate(pages.length - 1, null) }
    Shortcut { sequence: "PgDown"; onActivated: document.contentY = Math.min(Math.max(0, document.contentHeight - document.height), document.contentY + document.height * 0.9) }
    Shortcut { sequence: "PgUp"; onActivated: document.contentY = Math.max(0, document.contentY - document.height * 0.9) }
}
