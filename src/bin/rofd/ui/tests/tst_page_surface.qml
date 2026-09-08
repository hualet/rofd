import QtQuick
import QtTest
import ".."
TestCase {
    id: test
    name: "PageRenderReplies"
    when: windowShown
    QtObject {
        id: backend
        property int requests: 0
        signal render_ready(string payload)
        signal generation_changed()
        function request_page(page, scale, thumbnail) { requests++ }
    }
    Component { id: surface; PageSurface { width: 600; height: 800; bridge: backend; pageNumber: 2; renderScale: 1.25 } }
    function rasterFor(page) {
        for (let child of page.children)
            if (child.source !== undefined && child.status !== undefined) return child
        fail("Raster Image not found")
    }
    function test_owned_data_uri_decodes() {
        let page = createTemporaryObject(surface, test)
        let source = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=="
        backend.render_ready(JSON.stringify({page:2,scale:1.25,thumbnail:false,source:source,error:""}))
        let raster = rasterFor(page)
        tryCompare(raster, "status", Image.Ready)
        compare(raster.sourceSize.width, 1)
        compare(raster.sourceSize.height, 1)
        compare(page.failure, "")
    }
    function test_decode_failure_offers_retry() {
        let page = createTemporaryObject(surface, test)
        ignoreWarning(/.*Error decoding.*/)
        backend.render_ready(JSON.stringify({page:2,scale:1.25,thumbnail:false,source:"data:image/png;base64,YmFk",error:""}))
        tryCompare(rasterFor(page), "status", Image.Error)
        verify(page.failure.length > 0)
        let count = backend.requests
        page.request()
        compare(backend.requests, count + 1)
        compare(page.failure, "")
        compare(rasterFor(page).status, Image.Null)
    }
    function test_stale_scale_and_document_refresh() {
        backend.requests = 0
        let page = createTemporaryObject(surface, test)
        compare(backend.requests, 1)
        backend.render_ready(JSON.stringify({page:2,scale:1,thumbnail:false,source:"",error:"stale"}))
        verify(page.requested)
        compare(page.failure, "")
        backend.render_ready(JSON.stringify({page:2,scale:1.25,thumbnail:false,source:"",error:"page error"}))
        verify(!page.requested)
        compare(page.failure, "page error")
        backend.generation_changed()
        tryCompare(backend, "requests", 2)
        compare(page.failure, "")
        verify(page.requested)
    }
}
