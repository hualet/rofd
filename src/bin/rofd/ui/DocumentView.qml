import QtQuick
import QtQuick.Controls.Basic
import "ReaderMath.js" as MathUtil
Flickable {
    id: view
    objectName: "documentView"
    required property var bridge
    property var pages: []
    property real zoom: 1
    property var geometry: MathUtil.layout(pages, zoom)
    property var results: []
    property int activeResult: -1
    property bool restoring: false
    signal zoomRequested(real value)
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    contentWidth: Math.max(width, geometry.reduce(function(w, p) { return Math.max(w, p.width + 48) }, 0))
    contentHeight: geometry.length ? geometry[geometry.length - 1].top + geometry[geometry.length - 1].height + 24 : 0
    ScrollBar.vertical: ScrollBar { }
    ScrollBar.horizontal: ScrollBar { }
    function anchor() {
        let i = MathUtil.pageAt(geometry, contentY + height / 2)
        if (!geometry.length) return {page:0, fraction:0, horizontal:0.5}
        let p = geometry[i]
        return {page:i, fraction:(contentY + height / 2 - p.top) / p.height,
            horizontal: (contentX + width / 2) / contentWidth}
    }
    function restore(a) {
        if (!geometry.length) return
        let p = geometry[Math.min(a.page, geometry.length - 1)]
        contentY = Math.max(0, Math.min(contentHeight - height, p.top + a.fraction * p.height - height / 2))
        contentX = Math.max(0, Math.min(contentWidth - width, a.horizontal * contentWidth - width / 2))
    }
    function jump(index, region) {
        if (!geometry.length) return
        index = Math.max(0, Math.min(index, geometry.length - 1))
        let p = geometry[index]
        contentY = Math.max(0, Math.min(contentHeight - height, p.top + (region ? region.y * p.height - height / 3 : -24)))
        if (region) {
            let targetX = (contentWidth - p.width) / 2 + region.x * p.width
            if (targetX < contentX + 16 || targetX > contentX + width - 16)
                contentX = Math.max(0, Math.min(contentWidth - width, targetX - width / 4))
        }
        bridge.current_page = index
    }
    function syncCurrentPage() { if (geometry.length && !restoring) bridge.current_page = MathUtil.pageAt(geometry, contentY + height / 2) }
    onContentYChanged: syncCurrentPage()
    Repeater {
        model: view.pages.length
        Loader {
            required property int index
            property var modelData: view.geometry[index] || {top:0,width:0,height:0}
            x: (view.contentWidth - width) / 2; y: modelData.top
            width: modelData.width; height: modelData.height
            active: index >= MathUtil.pageAt(view.geometry, view.contentY) - 1
                && index <= MathUtil.pageAt(view.geometry, view.contentY + view.height) + 1
            sourceComponent: PageSurface {
                bridge: view.bridge; pageNumber: index
                renderScale: view.zoom * view.Screen.devicePixelRatio
                results: view.results; activeResult: view.activeResult
            }
        }
    }
    WheelHandler {
        acceptedModifiers: Qt.ControlModifier
        onWheel: function(event) {
            view.zoomRequested(view.zoom * (event.angleDelta.y > 0 ? 1.1 : 1 / 1.1))
            event.accepted = true
        }
    }
}
