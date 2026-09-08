import QtQuick
import QtTest
import ".."
TestCase {
    id: test
    name: "ContinuousDocument"
    when: windowShown
    width: 800; height: 600
    QtObject {
        id: backend
        property int current_page: 0
        property var requests: []
        signal render_ready(string payload)
        signal generation_changed()
        function request_page(page, scale, thumbnail) { requests.push({page:page,scale:scale}) }
    }
    Component {
        id: viewComponent
        DocumentView { width: 800; height: 600; bridge: backend }
    }
    function test_limited_requests_and_anchor() {
        backend.requests = []
        let view = createTemporaryObject(viewComponent, test)
        let pages = []
        for (let i = 0; i < 30; ++i) pages.push({width:600, height:i % 2 ? 400 : 800})
        view.pages = pages
        wait(50)
        verify(backend.requests.length > 0)
        verify(backend.requests.length <= 3, "Only visible and neighboring pages should render")
        view.jump(15, null)
        compare(backend.current_page, 15)
        let anchor = view.anchor()
        view.zoom = 2
        view.restore(anchor)
        let after = view.anchor()
        compare(after.page, anchor.page)
        fuzzyCompare(after.fraction, anchor.fraction, 0.001)
        verify(view.contentWidth > view.width)
    }
}
