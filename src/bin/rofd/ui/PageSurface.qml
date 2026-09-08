import QtQuick
import QtQuick.Controls.Basic
Rectangle {
    id: page
    required property var bridge
    required property int pageNumber
    property real renderScale: 1
    property bool thumbnail: false
    readonly property real requestedScale: Math.max(0.01, Math.min(16, Math.round(renderScale * 100) / 100))
    property bool requested: false
    property int requestTicket: 0
    property bool reduced: false
    property string failure: ""
    property var results: []
    property int activeResult: -1
    color: "white"
    border.color: "#dce1e8"
    Rectangle { x: 2; y: 3; width: parent.width; height: parent.height; color: "#100f172a"; z: -1 }
    function request() {
        if (raster.status === Image.Error) raster.source = ""
        requested = true
        failure = ""
        if (requestTicket > 0) bridge.cancel_render(requestTicket)
        requestTicket = bridge.request_page(pageNumber, requestedScale, thumbnail) || 0
    }
    onRenderScaleChanged: refresh.restart()
    Component.onCompleted: request()
    Component.onDestruction: if (requestTicket > 0) bridge.cancel_render(requestTicket)
    Timer { id: refresh; interval: 160; onTriggered: page.request() }
    Connections {
        target: page.bridge
        function onGeneration_changed() { raster.source = ""; refresh.restart() }
        function onRender_ready(payload) {
            let reply = JSON.parse(payload)
            if (reply.page !== page.pageNumber || reply.thumbnail !== page.thumbnail
                || Math.abs(reply.scale - page.requestedScale) > 0.001) return
            page.requested = false
            page.failure = reply.error || ""
            page.reduced = reply.reduced || false
            raster.source = reply.source || ""
        }
    }
    Image {
        id: raster
        anchors.fill: parent
        cache: false
        asynchronous: true
        fillMode: Image.Stretch
        onStatusChanged: {
            if (status === Image.Error) {
                page.requested = false
                page.failure = "页面图像解码失败，请重试"
            }
        }
    }
    Repeater {
        model: page.thumbnail ? [] : page.results
        Rectangle {
            required property var modelData
            required property int index
            visible: modelData.page === page.pageNumber
            x: modelData.x * page.width; y: modelData.y * page.height
            width: Math.max(2, modelData.width * page.width); height: Math.max(2, modelData.height * page.height)
            color: index === page.activeResult ? "#70ffb429" : "#50ffe16a"
            border.color: index === page.activeResult ? "#df9411" : "#cdb34d"
        }
    }
    BusyIndicator { anchors.centerIn: parent; width: 30; height: 30; visible: running; running: page.requested && raster.status !== Image.Ready }
    Column {
        anchors.centerIn: parent; width: parent.width - 32; spacing: 10
        visible: page.failure.length > 0
        Label { width: parent.width; text: "页面加载失败"; horizontalAlignment: Text.AlignHCenter; color: "#8c4650" }
        Label { width: parent.width; text: page.failure; textFormat: Text.PlainText; wrapMode: Text.Wrap; horizontalAlignment: Text.AlignHCenter; font.pixelSize: 11; visible: !page.thumbnail }
        Button { anchors.horizontalCenter: parent.horizontalCenter; text: "重试"; onClicked: page.request() }
    }
    Label {
        anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 8
        visible: page.reduced && !page.thumbnail; text: "大页面 · 已降低预览清晰度"; font.pixelSize: 11; color: "#707b8c"
    }
}
