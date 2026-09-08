import QtQuick
Canvas {
    id: icon
    property string name: ""
    property color ink: "#354156"
    implicitWidth: 18; implicitHeight: 18
    onNameChanged: requestPaint()
    onInkChanged: requestPaint()
    onPaint: {
        let ctx = getContext("2d")
        ctx.reset(); ctx.strokeStyle = ink; ctx.lineWidth = 1.5
        ctx.lineCap = "round"; ctx.lineJoin = "round"
        ctx.beginPath()
        if (name === "sidebar") {
            ctx.rect(2.5, 3, 13, 12); ctx.moveTo(7, 3); ctx.lineTo(7, 15)
        } else if (name === "previous" || name === "next") {
            let direction = name === "previous" ? -1 : 1
            ctx.moveTo(9 - direction * 2, 4); ctx.lineTo(9 + direction * 3, 9); ctx.lineTo(9 - direction * 2, 14)
        } else if (name === "plus" || name === "minus") {
            ctx.moveTo(3, 9); ctx.lineTo(15, 9)
            if (name === "plus") { ctx.moveTo(9, 3); ctx.lineTo(9, 15) }
        } else if (name === "up" || name === "down") {
            let direction = name === "up" ? -1 : 1
            ctx.moveTo(4, 9 - direction * 2); ctx.lineTo(9, 9 + direction * 3); ctx.lineTo(14, 9 - direction * 2)
        } else if (name === "close") {
            ctx.moveTo(4, 4); ctx.lineTo(14, 14); ctx.moveTo(14, 4); ctx.lineTo(4, 14)
        }
        ctx.stroke()
    }
}
