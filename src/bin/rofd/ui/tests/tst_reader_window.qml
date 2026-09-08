import QtQuick
import QtTest
TestCase {
    id: test
    name: "ReaderWindow"
    when: windowShown
    QtObject {
        id: ofdViewer
        property string document_title: "混合尺寸测试.ofd"
        property string pages_json: '[{"width":600,"height":800},{"width":800,"height":400},{"width":600,"height":800}]'
        property string search_json: "[]"
        property string search_status: ""
        property string error_message: ""
        property int page_count: 3
        property int current_page: 0
        property int generation: 1
        signal generation_changed()
        property bool busy: false
        property bool search_busy: false
        signal render_ready(string payload)
        function open_file(path) {}
        function poll() {}
        function request_page(page, scale, thumbnail) {}
        function search(query) {}
        function clear_error() {}
    }
    function test_open_document_starts_at_first_page_top() {
        let original = ofdViewer.pages_json
        ofdViewer.pages_json = "[]"
        let component = Qt.createComponent("../main_window.qml")
        let win = component.createObject(test)
        wait(100)
        ofdViewer.pages_json = original
        ofdViewer.generation++
        ofdViewer.generation_changed()
        wait(500)
        let view = findChild(win, "documentView")
        verify(view !== null)
        compare(findChild(win, "zoomInput").text, Math.round(win.zoom * 100) + "%")
        compare(view.contentY, 0)
        verify(view.contentWidth <= view.width + 1, "Fit width must accommodate mixed page widths")
        compare(ofdViewer.current_page, 0)
        win.close(); win.destroy()
    }
    function test_window_navigation() {
        let component = Qt.createComponent("../main_window.qml")
        compare(component.status, Component.Ready, component.errorString())
        let win = component.createObject(test)
        verify(win !== null, component.errorString())
        wait(150)
        let input = findChild(win, "zoomInput")
        win.requestActivate()
        tryCompare(win, "active", true)
        input.forceActiveFocus()
        input.text = "125%"
        keyClick(Qt.Key_Return)
        compare(win.zoom, 1.25)
        win.setZoom(2)
        compare(win.zoom, 2)
        win.width = 800
        wait(100)
        compare(win.zoom, 2)
        win.fit("page")
        verify(win.zoom >= 0.25 && win.zoom <= 4)
        win.showSearch()
        verify(win.sidebar)
        keyClick(Qt.Key_Escape)
        keyClick(Qt.Key_End, Qt.ControlModifier)
        compare(ofdViewer.current_page, 2)
        win.close()
        win.destroy()
    }
}
