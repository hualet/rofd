import QtQuick
import QtTest
import "../ReaderMath.js" as MathUtil
TestCase {
    name: "ReaderNavigation"
    function test_zoom_limits() {
        compare(MathUtil.clampZoom(0.1), 0.25)
        compare(MathUtil.clampZoom(7), 4)
        compare(MathUtil.clampZoom(1.25), 1.25)
    }
    function test_variable_page_geometry() {
        let pages = [{width:600,height:800},{width:800,height:400}]
        compare(MathUtil.layout(pages, 0.5)[1].top, 448)
        compare(MathUtil.pageAt(MathUtil.layout(pages, 0.5), 480), 1)
    }
    function test_results_wrap() {
        compare(MathUtil.nextResult(0,-1,3), 2)
        compare(MathUtil.nextResult(2,1,3), 0)
        compare(MathUtil.nextResult(-1,1,0), -1)
    }
}
