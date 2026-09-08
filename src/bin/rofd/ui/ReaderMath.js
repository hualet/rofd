.pragma library
function clampZoom(value) { return Math.max(0.25, Math.min(4, value)); }
function layout(pages, zoom) {
    let top = 24;
    return pages.map(function(page) {
        let item = {top: top, width: page.width * zoom, height: page.height * zoom};
        top += item.height + 24;
        return item;
    });
}
function pageAt(items, y) {
    for (let i = 0; i < items.length; ++i)
        if (y < items[i].top + items[i].height + 12) return i;
    return Math.max(0, items.length - 1);
}
function nextResult(current, direction, count) {
    return count ? (current + direction + count) % count : -1;
}
