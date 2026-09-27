// Draws Refr's app icon and writes resources/AppIcon.icns (and AppIcon.png).
//
//   swift scripts/make_icon.swift
//
// A red squircle holding a white page with a folded corner, lines of text and one
// highlighted line. Follows the macOS icon grid (824 pt body on a 1024 pt canvas).
// Coordinates are CoreGraphics', with y growing upward.

import AppKit
import CoreGraphics

func color(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255, alpha: alpha)
}

func drawIcon(size: CGFloat) -> CGImage {
    let space = CGColorSpace(name: CGColorSpace.sRGB)!
    let ctx = CGContext(
        data: nil, width: Int(size), height: Int(size), bitsPerComponent: 8, bytesPerRow: 0,
        space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.scaleBy(x: size / 1024, y: size / 1024)

    // Body: squircle with a soft drop shadow and a red gradient.
    let body = CGRect(x: 100, y: 100, width: 824, height: 824)
    let bodyPath = CGPath(roundedRect: body, cornerWidth: 185, cornerHeight: 185, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -12), blur: 28, color: color(0x000000, 0.30))
    ctx.addPath(bodyPath)
    ctx.setFillColor(color(0xC92C24))
    ctx.fillPath()
    ctx.restoreGState()

    ctx.saveGState()
    ctx.addPath(bodyPath)
    ctx.clip()
    let background = CGGradient(colorsSpace: space, colors: [color(0xEF5A4C), color(0xB61F18)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(background, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 100), options: [])

    // The page, with its top-right corner folded.
    let left: CGFloat = 300, right: CGFloat = 724, bottom: CGFloat = 215, top: CGFloat = 809, fold: CGFloat = 120
    let page = CGMutablePath()
    page.move(to: CGPoint(x: left, y: bottom))
    page.addLine(to: CGPoint(x: right, y: bottom))
    page.addLine(to: CGPoint(x: right, y: top - fold))
    page.addLine(to: CGPoint(x: right - fold, y: top))
    page.addLine(to: CGPoint(x: left, y: top))
    page.closeSubpath()
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 30, color: color(0x5A0B07, 0.45))
    ctx.addPath(page)
    ctx.setFillColor(color(0xFFFFFF))
    ctx.fillPath()
    ctx.restoreGState()

    let corner = CGMutablePath()
    corner.move(to: CGPoint(x: right - fold, y: top))
    corner.addLine(to: CGPoint(x: right - fold, y: top - fold + 14))
    corner.addQuadCurve(to: CGPoint(x: right - fold + 14, y: top - fold), control: CGPoint(x: right - fold, y: top - fold))
    corner.addLine(to: CGPoint(x: right, y: top - fold))
    corner.closeSubpath()
    ctx.addPath(corner)
    ctx.setFillColor(color(0xE3D9D8))
    ctx.fillPath()

    // Lines of text; the third is highlighted.
    let lines: [(CGFloat, CGFloat)] = [(655, 0.55), (585, 0.86), (515, 0.78), (445, 0.86), (375, 0.66), (305, 0.48)]
    for (index, (y, fraction)) in lines.enumerated() {
        let width = (right - left - 120) * fraction
        let bar = CGRect(x: left + 60, y: y - 14, width: width, height: 28)
        if index == 2 {
            ctx.addPath(CGPath(roundedRect: bar.insetBy(dx: -18, dy: -18), cornerWidth: 10, cornerHeight: 10, transform: nil))
            ctx.setFillColor(color(0xFFCA28, 0.85))
            ctx.fillPath()
        }
        ctx.addPath(CGPath(roundedRect: bar, cornerWidth: 14, cornerHeight: 14, transform: nil))
        ctx.setFillColor(color(index == 0 ? 0x3A3A3A : 0xB9B9B9))
        ctx.fillPath()
    }

    // Top highlight on the body.
    let sheen = CGGradient(colorsSpace: space, colors: [color(0xFFFFFF, 0.14), color(0xFFFFFF, 0)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(sheen, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 640), options: [])
    ctx.restoreGState()

    return ctx.makeImage()!
}

func writePNG(_ image: CGImage, to url: URL) {
    let rep = NSBitmapImageRep(cgImage: image)
    try! rep.representation(using: .png, properties: [:])!.write(to: url)
}

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let resources = root.appendingPathComponent("resources")
try! FileManager.default.createDirectory(at: resources, withIntermediateDirectories: true)
let iconset = FileManager.default.temporaryDirectory.appendingPathComponent("RefrIcon.iconset")
try? FileManager.default.removeItem(at: iconset)
try! FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)

for base in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = CGFloat(base * scale)
        let name = scale == 1 ? "icon_\(base)x\(base).png" : "icon_\(base)x\(base)@2x.png"
        writePNG(drawIcon(size: pixels), to: iconset.appendingPathComponent(name))
    }
}
writePNG(drawIcon(size: 1024), to: resources.appendingPathComponent("AppIcon.png"))

let output = resources.appendingPathComponent("AppIcon.icns")
let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", iconset.path, "-o", output.path]
try! iconutil.run()
iconutil.waitUntilExit()
print("wrote \(output.path)")
