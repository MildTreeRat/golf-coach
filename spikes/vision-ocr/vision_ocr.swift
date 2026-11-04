// vision_ocr.swift: M33's Apple Vision reader. Spike code (the plan's rule 11): nothing ships from
// here, and M38's plugin takes what this learned rather than necessarily this file.
//
//   swiftc -swift-version 5 -O spikes/vision-ocr/vision_ocr.swift -o target/m33/vision-ocr
//
//   vision-ocr decode IN OUT.png         IN (HEIC, JPEG or PNG) to an upright 8-bit sRGB PNG (call 2)
//   vision-ocr read IMAGE [--config C0]  Vision's boxes as JSON in screen::TextBox's keys (call 4),
//                                        each with its rgb (call 5)
//   vision-ocr glyph IMAGE BOXES.json    the rgb list for the boxes given, lab-images glyph's twin
//   vision-ocr rectify IN.png OUT.png    the phone's rectifier stand-in (call 7), as JSON and an image
//   vision-ocr selftest [DIR]            the geometry proven with no photo; DIR keeps its images
//
// One file built by swiftc rather than a package, because a package would need a manifest and a
// build directory for what is five subcommands and a test.

import CoreGraphics
import CoreImage
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers
import Vision

struct Failure: Error, CustomStringConvertible {
    let description: String
    init(_ description: String) { self.description = description }
}

// MARK: - What every output says about where it came from (rule 8)

/// The same spelling `probe.py check` prints, `26.2 (25C56)`, so the two can be compared by eye.
func osString() -> String {
    let v = ProcessInfo.processInfo.operatingSystemVersion
    var size = 0
    sysctlbyname("kern.osversion", nil, &size, nil, 0)
    var buffer = [CChar](repeating: 0, count: max(size, 1))
    sysctlbyname("kern.osversion", &buffer, &size, nil, 0)
    let version = v.patchVersion == 0
        ? "\(v.majorVersion).\(v.minorVersion)"
        : "\(v.majorVersion).\(v.minorVersion).\(v.patchVersion)"
    return "\(version) (\(String(cString: buffer)))"
}

// MARK: - JSON, written by hand

// By hand rather than JSONEncoder, for two things `boxes/` needs once it is committed: the keys in
// the order call 4 lists them, and one box to a line, so a re-measure diffs by the box. A Swift
// Double's `description` is the shortest text that round-trips, which is what lets rule 10 compare
// with `==` after the JSON trip, and call 4's "unrounded" holds.
func jsonNumber(_ value: Double) -> String {
    precondition(value.isFinite, "a box coordinate that is not finite has no JSON spelling")
    return value.description
}

func jsonString(_ value: String) throws -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = .withoutEscapingSlashes
    return String(decoding: try encoder.encode(value), as: UTF8.self)
}

func jsonRGB(_ rgb: [Double]?) -> String {
    guard let rgb else { return "null" }
    return "[" + rgb.map(jsonNumber).joined(separator: ", ") + "]"
}

func writeStdout(_ text: String) {
    FileHandle.standardOutput.write(Data((text + "\n").utf8))
}

// MARK: - Images

let sRGB = CGColorSpace(name: CGColorSpace.sRGB)!
let displayP3 = CGColorSpace(name: CGColorSpace.displayP3)!
let ciContext = CIContext()

/// A file's image as stored: no orientation applied and no colour matched, since ImageIO's
/// `CGImageSourceCreateImageAtIndex` does neither. So the coordinates Vision reports on this image
/// are the coordinates of the bytes in the file, which is what `screen::TextBox` says they are.
func loadCGImage(_ url: URL) throws -> CGImage {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
        let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
    else { throw Failure("cannot read an image from \(url.path)") }
    return image
}

func imageProperties(_ url: URL) -> [String: Any] {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any]
    else { return [:] }
    return properties
}

func writeImage(_ image: CGImage, to url: URL, type: UTType, properties: [CFString: Any] = [:]) throws {
    try FileManager.default.createDirectory(
        at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    guard
        let destination = CGImageDestinationCreateWithURL(
            url as CFURL, type.identifier as CFString, 1, nil)
    else { throw Failure("this Mac cannot write \(type.identifier) to \(url.path)") }
    CGImageDestinationAddImage(destination, image, properties as CFDictionary)
    guard CGImageDestinationFinalize(destination) else {
        throw Failure("writing \(url.path) failed")
    }
}

/// Call 2: one decode path, ImageIO through Core Image, as the phone's plugin would. It colour
/// matches on the way, so a Display P3 photo comes out converted, which is the measurement M38 needs
/// and M42's `rgb24` numbers did not make.
///
/// Rendered to a CGImage and written with no properties, so nothing of the source's metadata reaches
/// the PNG. A stale Orientation is what would hurt, as anything honouring it would turn the pixels a
/// second time, and `selftest` pins its absence on a tagged file. It is not why `writePNGRepresentation`
/// was passed over: tried on the same file, Core Image had already dropped the tag on applying it.
func decode(input: URL, output: URL) throws -> (width: Int, height: Int) {
    guard let image = CIImage(contentsOf: input, options: [.applyOrientationProperty: true]) else {
        throw Failure("Core Image cannot open \(input.path)")
    }
    guard
        let rendered = ciContext.createCGImage(
            image, from: image.extent, format: .RGBA8, colorSpace: sRGB)
    else { throw Failure("Core Image cannot render \(input.path)") }
    try writeImage(rendered, to: output, type: .png)
    return (rendered.width, rendered.height)
}

/// The bytes of an image exactly as the file holds them, for the self-test's scans and for `rgb`. No
/// colour matching, which is what call 5 asks: on a PNG tagged Display P3, reading through a
/// `CGContext.draw` into sRGB moved 489 of 500 boxes off Rust's answer, and this moves none. On macOS
/// 26.2 ImageIO hands back both `decode`'s sRGB PNG and Rust's untagged RGB one as 32 bits a pixel with
/// the fourth byte last, and names the untagged one sRGB. The stride is read off the image all the same.
struct Pixels {
    let data: Data
    let width: Int
    let height: Int
    let bytesPerRow: Int
    let bytesPerPixel: Int

    init(_ image: CGImage) throws {
        guard let provider = image.dataProvider, let bytes = provider.data else {
            throw Failure("the image has no readable bytes")
        }
        guard image.bitsPerComponent == 8, image.bitsPerPixel >= 24 else {
            throw Failure("expected 8-bit RGB, found \(image.bitsPerPixel) bits per pixel")
        }
        switch image.alphaInfo {
        case .first, .premultipliedFirst, .noneSkipFirst:
            throw Failure("alpha leads this image's bytes, and the scans read them as RGB")
        default:
            break
        }
        // Little-endian would put blue first with nothing else about the image looking wrong, and a
        // lit letter is told from an unlit one by its blue.
        let order = image.bitmapInfo.intersection(.byteOrderMask)
        if order == .byteOrder16Little || order == .byteOrder32Little {
            throw Failure("this image's bytes are little-endian, and the scans read them as RGB")
        }
        data = bytes as Data
        width = image.width
        height = image.height
        bytesPerRow = image.bytesPerRow
        bytesPerPixel = image.bitsPerPixel / 8
    }

    func rgb(_ x: Int, _ y: Int) -> (Int, Int, Int) {
        let at = y * bytesPerRow + x * bytesPerPixel
        return (Int(data[at]), Int(data[at + 1]), Int(data[at + 2]))
    }
}

// MARK: - read: Vision, at C0

struct Box {
    let text: String
    let x: Double
    let y: Double
    let width: Double
    let height: Double
    let confidence: Double
    /// `TextBox::rgb`. Filled after the mapping, by `glyphRGB` in the image the coordinates are in,
    /// and `nil` is both "not measured yet" and "holds no pixel", as `null` is in the JSON.
    var rgb: [Double]? = nil
}

/// One axis of a box clipped to an image `extent` pixels long, as `[start, end)` pixel indices, or
/// `nil` when that is empty: `crates/ocr/src/recognizer.rs::clipped`, line for line. So a box with no
/// width at a fractional x still holds one column, because the floor and the ceil differ, and that is
/// the Rust's behaviour rather than something to tidy here.
func clipped(_ start: Double, _ length: Double, _ extent: Int) -> (low: Int, high: Int)? {
    let low = max(start, 0).rounded(.down)
    let high = min((start + length).rounded(.up), Double(extent))
    if low >= high { return nil }
    return (Int(low), Int(high))
}

/// Call 5: `crates/ocr/src/recognizer.rs::glyph_rgb`, which is `TextBox::rgb`'s definition. The mean of
/// the brightest quarter of the box's pixels, the ranks from `n * 3 / 4` to the end, by `r + g + b`
/// ascending with ties in row-major order.
///
/// Rust gets its tie rule from a stable sort. Swift's `sort` is not documented as stable, so the tie
/// rule is in the key instead: the sum above the pixel's row-major index, which makes every key
/// distinct and the order the same whatever the sort does. A box holds at most the image's pixels, so
/// the index fits below bit 40 and the sum, at most 765, above it.
///
/// The running totals are Doubles of integers below 2^53, as in Rust, so the division is the only
/// rounding and both languages do the same one.
func glyphRGB(_ pixels: Pixels, x: Double, y: Double, width: Double, height: Double) -> [Double]? {
    guard let columns = clipped(x, width, pixels.width),
        let rows = clipped(y, height, pixels.height)
    else { return nil }
    let across = columns.high - columns.low
    var keys: [UInt64] = []
    keys.reserveCapacity(across * (rows.high - rows.low))
    var index: UInt64 = 0
    for row in rows.low..<rows.high {
        for column in columns.low..<columns.high {
            let (r, g, b) = pixels.rgb(column, row)
            keys.append(UInt64(r + g + b) << 40 | index)
            index += 1
        }
    }
    keys.sort()
    let brightest = keys[(keys.count * 3 / 4)...]
    var sum = [0.0, 0.0, 0.0]
    for key in brightest {
        let rank = Int(key & ((1 << 40) - 1))
        let (r, g, b) = pixels.rgb(columns.low + rank % across, rows.low + rank / across)
        sum[0] += Double(r)
        sum[1] += Double(g)
        sum[2] += Double(b)
    }
    let n = Double(brightest.count)
    return sum.map { $0 / n }
}

/// Call 4's C0, and finding 8 is why `customWords` is empty: Apple's reference says they are ignored
/// with correction off, so setting them here would claim something C0 does not do. C3 turns both on.
///
/// The revision is set first because it decides which languages the request can take, and pinned
/// rather than left at the default so a later macOS cannot change the recognizer under a measurement.
func recognize(_ image: CGImage) throws -> (boxes: [Box], revision: Int) {
    let request = VNRecognizeTextRequest()
    request.revision = VNRecognizeTextRequestRevision3
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["en-US"]
    request.automaticallyDetectsLanguage = false
    request.usesLanguageCorrection = false
    request.customWords = []
    try VNImageRequestHandler(cgImage: image, orientation: .up, options: [:]).perform([request])

    let width = Double(image.width)
    let height = Double(image.height)
    var boxes: [Box] = []
    for observation in request.results ?? [] {
        guard let top = observation.topCandidates(1).first else { continue }
        let rect = observation.boundingBox
        // Vision's normalized box has its origin at the bottom left and `TextBox` at the top left,
        // so y is the flip of the box's far edge. The self-test exists to catch this line being wrong.
        boxes.append(
            Box(
                text: top.string,
                x: Double(rect.minX) * width,
                y: (1.0 - Double(rect.maxY)) * height,
                width: Double(rect.width) * width,
                height: Double(rect.height) * height,
                confidence: Double(top.confidence)))
    }
    return (boxes, request.revision)
}

func readJSON(path: String, config: String) throws -> String {
    let image = try loadCGImage(URL(fileURLWithPath: path))
    var (boxes, revision) = try recognize(image)
    // Here rather than in `recognize`, because P12's rungs read one image and report in another (C2's
    // upscale, C4's crops), and call 10 says which image each rung's `rgb` is measured in.
    let pixels = try Pixels(image)
    for index in boxes.indices {
        let box = boxes[index]
        boxes[index].rgb = glyphRGB(pixels, x: box.x, y: box.y, width: box.width, height: box.height)
    }
    var lines: [String] = []
    lines.append("{")
    lines.append("  \"image\": \(try jsonString(path)),")
    lines.append("  \"width\": \(image.width),")
    lines.append("  \"height\": \(image.height),")
    lines.append("  \"config\": \(try jsonString(config)),")
    lines.append("  \"revision\": \(revision),")
    lines.append("  \"os\": \(try jsonString(osString())),")
    if boxes.isEmpty {
        lines.append("  \"boxes\": []")
    } else {
        lines.append("  \"boxes\": [")
        for (index, box) in boxes.enumerated() {
            let comma = index + 1 < boxes.count ? "," : ""
            let row =
                "{\"text\": \(try jsonString(box.text)), \"x\": \(jsonNumber(box.x)), "
                + "\"y\": \(jsonNumber(box.y)), \"width\": \(jsonNumber(box.width)), "
                + "\"height\": \(jsonNumber(box.height)), "
                + "\"confidence\": \(jsonNumber(box.confidence)), \"rgb\": \(jsonRGB(box.rgb))}"
            lines.append("    \(row)\(comma)")
        }
        lines.append("  ]")
    }
    lines.append("}")
    return lines.joined(separator: "\n")
}

// MARK: - glyph: the seam P3 holds to Rust

/// Only a box's rectangle, so that the same file goes to both languages, and `read`'s output is such a
/// file. What this is for is the boxes Vision never returns, which `read` therefore cannot try: off
/// each edge, wholly outside, no width, and the small counts where `n * 3 / 4` truncates. `read` proves
/// the wiring and the photos' pixels, and this proves the rest of the definition against `lab-images
/// glyph`. JSONDecoder's way with a long decimal is not what is on trial, so a file meant for this
/// should hold coordinates a short binary fraction spells exactly.
struct BoxesFile: Decodable {
    struct Rect: Decodable {
        let x: Double
        let y: Double
        let width: Double
        let height: Double
    }
    let boxes: [Rect]
}

func glyphJSON(image: String, boxes: String) throws -> String {
    let pixels = try Pixels(try loadCGImage(URL(fileURLWithPath: image)))
    let file = try JSONDecoder().decode(
        BoxesFile.self, from: try Data(contentsOf: URL(fileURLWithPath: boxes)))
    let rows = file.boxes.map {
        jsonRGB(glyphRGB(pixels, x: $0.x, y: $0.y, width: $0.width, height: $0.height))
    }
    return "[\n" + rows.map { "  \($0)" }.joined(separator: ",\n") + "\n]"
}

// MARK: - rectify: the phone's rectifier stand-in

/// `ocr::prepare::UNCROPPED_NOTE`'s text, verbatim (call 3), so that a `vision` read which fell back
/// tells the parser what a `lab` read that fell back tells it.
let uncroppedNote = "screen outline not found - parsing the photo uncropped"

struct Rectangle {
    let confidence: Double
    /// Top left, top right, bottom right, bottom left, in pixels from the image's top left: the order
    /// and the origin of the lab's `quad.corners`, so the two can be set side by side.
    let corners: [CGPoint]
    /// The same four as Core Image wants them: pixels, with the origin at the bottom left.
    let imageCorners: [CGPoint]
    let area: Double
}

/// A four-sided figure's area by the shoelace formula, since a skewed quad is not its bounding box.
func shoelace(_ corners: [CGPoint]) -> Double {
    var twice = 0.0
    for (index, a) in corners.enumerated() {
        let b = corners[(index + 1) % corners.count]
        twice += Double(a.x) * Double(b.y) - Double(b.x) * Double(a.y)
    }
    return abs(twice) / 2
}

/// Call 7's values, and they are not tuned: a wrong answer here is P6's finding, not a bug to chase,
/// because VisionKit's scanner is what the phone would really use and M38 measures that. Each is set
/// even where it is the default (`quadratureTolerance`), and the revision is pinned, so that a later
/// macOS cannot change the request under a measurement. Largest first, and the sort is stable by index
/// so that equal areas keep Vision's order.
///
/// Measured on the six pasted stills (macOS 26.2): a rectangle on one, and that one the replay panel,
/// holding no tile. Neither of the monitor's side edges is in any of those frames, so the screen has no
/// four corners to find. That is P6's finding, and the reason `vision` is not an independent arm here.
func detectRectangles(_ image: CGImage) throws -> (rectangles: [Rectangle], revision: Int) {
    let request = VNDetectRectanglesRequest()
    request.revision = VNDetectRectanglesRequestRevision1
    request.minimumAspectRatio = 0.4
    request.maximumAspectRatio = 0.9
    request.minimumSize = 0.3
    request.quadratureTolerance = 30
    request.minimumConfidence = 0.5
    request.maximumObservations = 0
    try VNImageRequestHandler(cgImage: image, orientation: .up, options: [:]).perform([request])

    let width = CGFloat(image.width)
    let height = CGFloat(image.height)
    let rectangles = (request.results ?? []).map { observation -> Rectangle in
        let normalized = [
            observation.topLeft, observation.topRight, observation.bottomRight, observation.bottomLeft,
        ]
        let corners = normalized.map { CGPoint(x: $0.x * width, y: (1 - $0.y) * height) }
        return Rectangle(
            confidence: Double(observation.confidence),
            corners: corners,
            imageCorners: normalized.map { CGPoint(x: $0.x * width, y: $0.y * height) },
            area: shoelace(corners))
    }
    let ranked = rectangles.enumerated().sorted {
        $0.element.area != $1.element.area ? $0.element.area > $1.element.area : $0.offset < $1.offset
    }
    return (ranked.map { $0.element }, request.revision)
}

/// `CIPerspectiveCorrection` on the rectangle's corners, rendered to sRGB as `decode` is. Core Image
/// resamples, so `rgb` in this image is not the file's pixels, and a lit letter's colour here has been
/// through that as well as through `decode`'s conversion (P9's lit table).
func perspectiveCorrected(_ input: URL, _ rectangle: Rectangle) throws -> CGImage {
    guard let source = CIImage(contentsOf: input, options: [.applyOrientationProperty: true]) else {
        throw Failure("Core Image cannot open \(input.path)")
    }
    guard let filter = CIFilter(name: "CIPerspectiveCorrection") else {
        throw Failure("this Mac has no CIPerspectiveCorrection")
    }
    let keys = ["inputTopLeft", "inputTopRight", "inputBottomRight", "inputBottomLeft"]
    filter.setValue(source, forKey: kCIInputImageKey)
    for (key, corner) in zip(keys, rectangle.imageCorners) {
        filter.setValue(CIVector(cgPoint: corner), forKey: key)
    }
    guard let output = filter.outputImage,
        let rendered = ciContext.createCGImage(
            output, from: output.extent, format: .RGBA8, colorSpace: sRGB)
    else { throw Failure("Core Image cannot rectify \(input.path)") }
    return rendered
}

/// Call 7 and call 3's `vision` row. Reads `decode`'s PNG, as every Swift step does (call 2). Where no
/// rectangle is found the output is the input's own bytes rather than a second encoding, so that
/// `vision` falling back is `raw` exactly and a difference between the two arms can only be the note.
///
/// Every rectangle Vision returned is printed under `candidates`, beyond call 7's three keys, because
/// "the wrong one was kept" is a finding that needs the ones passed over.
func rectifyJSON(input: URL, output: URL) throws -> String {
    let image = try loadCGImage(input)
    let (rectangles, revision) = try detectRectangles(image)
    let whole = Double(image.width) * Double(image.height)

    func corners(_ rectangle: Rectangle) -> String {
        let points = rectangle.corners.map { "[\(jsonNumber(Double($0.x))), \(jsonNumber(Double($0.y)))]" }
        return "[" + points.joined(separator: ", ") + "]"
    }
    func size(_ width: Int, _ height: Int) -> String { "{\"width\": \(width), \"height\": \(height)}" }

    let found = rectangles.first
    var outputSize = (image.width, image.height)
    if let found {
        let rendered = try perspectiveCorrected(input, found)
        try writeImage(rendered, to: output, type: .png)
        outputSize = (rendered.width, rendered.height)
    } else {
        try FileManager.default.createDirectory(
            at: output.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data(contentsOf: input).write(to: output)
    }
    let candidates = rectangles.map {
        "{\"confidence\": \(jsonNumber($0.confidence)), \"area_fraction\": \(jsonNumber($0.area / whole)), "
            + "\"corners\": \(corners($0))}"
    }
    return [
        "{",
        "  \"found\": \(found != nil),",
        "  \"confidence\": \(found.map { jsonNumber($0.confidence) } ?? "null"),",
        "  \"corners\": \(found.map(corners) ?? "null"),",
        "  \"area_fraction\": \(found.map { jsonNumber($0.area / whole) } ?? "null"),",
        "  \"photo\": \(size(image.width, image.height)),",
        "  \"image\": \(size(outputSize.0, outputSize.1)),",
        "  \"notes\": \(found == nil ? "[\(try jsonString(uncroppedNote))]" : "[]"),",
        "  \"revision\": \(revision),",
        "  \"os\": \(try jsonString(osString())),",
        "  \"candidates\": [\(candidates.joined(separator: ", "))]",
        "}",
    ].joined(separator: "\n")
}

// MARK: - selftest

/// Where the self-test draws. Off-centre on both axes on purpose: `selftest` refuses a word whose
/// mirror image would still land within tolerance, so the fixture cannot be blind to a flip. The
/// frame is not square, so swapping W and H in the mapping moves every box too.
let selftestWidth = 1600
let selftestHeight = 1000

struct Word {
    let text: String
    let size: CGFloat
    /// The top left of the word's ink, in pixels from the top left of the image.
    let x: CGFloat
    let y: CGFloat
    /// Whether the box's centre is held to `tolerance`. A word that is not must still come back with
    /// the right text and with the ink's centre inside its box, which a flip would fail just the same.
    var held = true
}

// Five words are held and two are only printed, and the split is measured, not designed. On this
// fixture's first run `CARRY`, `BACKSPIN`, `4199`, `R` and `TOTAL` sat inside the limit, and `9899` and
// `182.4` did not (dcy 4.75 and 11.63 against 3.5). Vision's box runs long below the ink: on a word with
// a point, 8-16% of the ink's height at 23-117 px (`182.4`, `102.5`), and on a four-digit integer usually
// under 7% but not reliably, since `9899` at this size gave dcy 0.80 at (300, 200) and 8.08 at
// (1040, 200). Moving a word until it passes would be fitting that noise, and a wider limit would make
// the plan's one number into two, so they stay here as rows, under the weaker check that still fails
// on a flip. Their table rows are the record that real values get boxes like these (P9).
let selftestWords = [
    Word(text: "CARRY", size: 64, x: 120, y: 70),
    Word(text: "9899", size: 96, x: 1040, y: 110, held: false),
    Word(text: "BACKSPIN", size: 64, x: 90, y: 250),
    Word(text: "4199", size: 96, x: 930, y: 690),
    Word(text: "R", size: 96, x: 150, y: 780),
    Word(text: "TOTAL", size: 64, x: 1240, y: 880),
    Word(text: "182.4", size: 96, x: 500, y: 690, held: false),
]

struct DrawnWord {
    let text: String
    let rect: CGRect
    let held: Bool
}

/// Draws the words white on near-black, and returns each one's ink rectangle from Core Text's glyph
/// paths, which is the box a perfect detector would report.
func drawSelftestImage() throws -> (CGImage, [DrawnWord]) {
    let width = selftestWidth
    let height = selftestHeight
    guard
        let context = CGContext(
            data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
            space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw Failure("no bitmap context") }
    context.setFillColor(CGColor(srgbRed: 18.0 / 255, green: 20.0 / 255, blue: 28.0 / 255, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))

    var drawn: [DrawnWord] = []
    for word in selftestWords {
        let font = CTFontCreateWithName("Helvetica-Bold" as CFString, word.size, nil)
        let attributes: [NSAttributedString.Key: Any] = [
            NSAttributedString.Key(kCTFontAttributeName as String): font,
            NSAttributedString.Key(kCTForegroundColorAttributeName as String):
                CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 1),
        ]
        let line = CTLineCreateWithAttributedString(
            NSAttributedString(string: word.text, attributes: attributes))
        // Relative to the baseline's origin, y up.
        let ink = CTLineGetBoundsWithOptions(line, .useGlyphPathBounds)
        context.textPosition = CGPoint(
            x: word.x - ink.minX,
            y: CGFloat(height) - word.y - ink.maxY)
        CTLineDraw(line, context)
        drawn.append(
            DrawnWord(
                text: word.text,
                rect: CGRect(x: word.x, y: word.y, width: ink.width, height: ink.height),
                held: word.held))
    }
    guard let image = context.makeImage() else { throw Failure("no image from the context") }
    return (image, drawn)
}

/// The bright pixels' bounds around a drawn word, in the file's own rows. This is what keeps the test
/// honest about y: if the drawing code above had the flip wrong in the same way `recognize` did, the
/// two would cancel and a comparison of box with rectangle alone would pass. Row 0 of a PNG's bytes is
/// the top of the picture whatever Core Graphics thinks, and Rust's `image` crate reads it the same.
func inkBounds(in pixels: Pixels, around rect: CGRect) -> CGRect? {
    let margin = 12
    let x0 = max(Int(rect.minX) - margin, 0)
    let x1 = min(Int(rect.maxX) + margin, pixels.width - 1)
    let y0 = max(Int(rect.minY) - margin, 0)
    let y1 = min(Int(rect.maxY) + margin, pixels.height - 1)
    guard x0 <= x1, y0 <= y1 else { return nil }
    var minX = Int.max
    var minY = Int.max
    var maxX = Int.min
    var maxY = Int.min
    for y in y0...y1 {
        for x in x0...x1 {
            let (r, g, b) = pixels.rgb(x, y)
            if r + g + b > 3 * 128 {
                minX = min(minX, x)
                maxX = max(maxX, x)
                minY = min(minY, y)
                maxY = max(maxY, y)
            }
        }
    }
    guard minX != Int.max else { return nil }
    return CGRect(x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1)
}

/// The plan's limit: 5% of the drawn rectangle's height, held on the box's **centre** and not on its
/// edges, because `boundingBox` is not the ink's rectangle and no edge can hold it. Measured on this
/// fixture's shapes at 32-160 pt in a 1600x1000 frame (macOS 26.2, revision 3): nearly every edge sits
/// outside the ink, typically 2-7 px and up to 45, and the coordinates step by about 2.33 px on this
/// frame, which is already most of the limit on a 46 px word. Padding is about size and the question
/// here is where, and the bugs this exists to catch move the centre far more than this: a flipped y by
/// 812 px on `CARRY`, `minY` for `maxY` by 57, a swapped W and H by 86 and 57.
let tolerance: CGFloat = 0.05

func compareWords(_ boxes: [Box], _ drawn: [DrawnWord], label: String, verbose: Bool) -> [String] {
    var failures: [String] = []
    if verbose {
        print(
            "  word       drawn x,y,w,h                 dx      dy      dr      db"
                + "  |   dcx     dcy   limit")
    }
    for word in drawn {
        guard let box = boxes.first(where: { $0.text == word.text }) else {
            let found = boxes.map { "'\($0.text)'" }.joined(separator: " ")
            failures.append("\(label): '\(word.text)' did not come back; Vision returned \(found)")
            continue
        }
        let r = word.rect
        let limit = tolerance * r.height
        let centreX = CGFloat(box.x + box.width / 2) - r.midX
        let centreY = CGFloat(box.y + box.height / 2) - r.midY
        if verbose {
            let rectText = String(
                format: "%7.1f %7.1f %6.1f %5.1f", r.minX, r.minY, r.width, r.height)
            let edges = String(
                format: "%6.2f  %6.2f  %6.2f  %6.2f",
                CGFloat(box.x) - r.minX, CGFloat(box.y) - r.minY,
                CGFloat(box.x + box.width) - r.maxX, CGFloat(box.y + box.height) - r.maxY)
            let centre = String(format: "%6.2f  %6.2f  %6.2f", centreX, centreY, limit)
            let name = word.text.padding(toLength: 9, withPad: " ", startingAt: 0)
            print("  \(name) \(rectText)  \(edges)  |  \(centre)\(word.held ? "" : "  not held")")
        }
        if word.held {
            if abs(centreX) > limit || abs(centreY) > limit {
                failures.append(
                    "\(label): '\(word.text)' has its centre off by (\(centreX), \(centreY)), "
                        + "and the limit is \(limit)")
            }
        } else if abs(CGFloat(box.x) + CGFloat(box.width) / 2 - r.midX) > CGFloat(box.width) / 2
            || abs(CGFloat(box.y) + CGFloat(box.height) / 2 - r.midY) > CGFloat(box.height) / 2
        {
            failures.append("\(label): '\(word.text)' has a box that misses the ink's centre")
        }
    }
    return failures
}

/// A fixture that a flip could not move out of tolerance would prove nothing about flips.
func checkFixtureSeesFlips(_ drawn: [DrawnWord]) -> [String] {
    var failures: [String] = []
    for word in drawn {
        let r = word.rect
        let limit = tolerance * r.height
        let mirroredY = CGFloat(selftestHeight) - r.maxY
        let mirroredX = CGFloat(selftestWidth) - r.maxX
        if abs(mirroredY - r.minY) <= limit || abs(mirroredX - r.minX) <= limit {
            failures.append("fixture: '\(word.text)' sits on the centre line, so a flip would not move it")
        }
    }
    return failures
}

/// The stored image of an upright picture whose file says Orientation 6: turned 90 degrees
/// anticlockwise, so that a reader honouring the tag turns it back. This is how a phone's upright
/// portrait is stored, and the originals are HEICs from a phone, though the pasted copies carry no tag.
func storedForOrientation6(_ upright: CGImage) throws -> CGImage {
    let width = upright.width
    let height = upright.height
    guard
        let context = CGContext(
            data: nil, width: height, height: width, bitsPerComponent: 8, bytesPerRow: 0,
            space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw Failure("no bitmap context") }
    context.translateBy(x: CGFloat(height), y: 0)
    context.rotate(by: .pi / 2)
    context.draw(upright, in: CGRect(x: 0, y: 0, width: width, height: height))
    guard let image = context.makeImage() else { throw Failure("no image from the context") }
    return image
}

func solidImage(_ rgb: (UInt8, UInt8, UInt8), space: CGColorSpace) throws -> CGImage {
    let side = 32
    var bytes = [UInt8](repeating: 255, count: side * side * 4)
    for i in stride(from: 0, to: bytes.count, by: 4) {
        bytes[i] = rgb.0
        bytes[i + 1] = rgb.1
        bytes[i + 2] = rgb.2
    }
    // Through a data provider, which holds its own copy. A CGContext over `&bytes` keeps a pointer that
    // is good only for the initializer's call: this was first written that way, read back the right
    // colours on one build, and read zeros on the next, after a change elsewhere in the file.
    guard let provider = CGDataProvider(data: Data(bytes) as CFData),
        let image = CGImage(
            width: side, height: side, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: side * 4,
            space: space, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
            provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    else { throw Failure("no solid image") }
    return image
}

/// Display P3 to sRGB by arithmetic rather than through a framework, so that the check is not Core
/// Image agreeing with itself. The two share a transfer curve and a white point, and the matrix is
/// the standard linear one between their primaries.
func p3ToSRGB(_ rgb: (UInt8, UInt8, UInt8)) -> [Double] {
    func toLinear(_ value: UInt8) -> Double {
        let c = Double(value) / 255
        return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
    }
    func fromLinear(_ value: Double) -> Double {
        let c = min(max(value, 0), 1)
        return 255 * (c <= 0.0031308 ? c * 12.92 : 1.055 * pow(c, 1 / 2.4) - 0.055)
    }
    let (r, g, b) = (toLinear(rgb.0), toLinear(rgb.1), toLinear(rgb.2))
    return [
        fromLinear(1.2249401763 * r - 0.2249401763 * g),
        fromLinear(-0.0420569547 * r + 1.0420569547 * g),
        fromLinear(-0.0196375546 * r - 0.0786360456 * g + 1.0982736002 * b),
    ]
}

/// A bright skewed panel on a dark frame, with a colour in three of its corners, and those corners in
/// pixels from the top left. Three because any two leave one wrong answer unseen: red and blue on the
/// diagonal survive swapping the top right with the bottom left, which is a transposed image. A dark
/// panel sits inside it, because call 7 keeps the largest for the reason that a screen holds panels of
/// its own, and the photos may never offer Vision that choice.
func drawSkewedPanel() throws -> (CGImage, [CGPoint]) {
    let width = selftestWidth
    let height = selftestHeight
    guard
        let context = CGContext(
            data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
            space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw Failure("no bitmap context") }
    context.setFillColor(CGColor(srgbRed: 0.07, green: 0.08, blue: 0.11, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))

    let corners = [
        CGPoint(x: 320, y: 140), CGPoint(x: 1290, y: 230), CGPoint(x: 1230, y: 860), CGPoint(x: 360, y: 790),
    ]
    func fill(_ points: [CGPoint], _ r: CGFloat, _ g: CGFloat, _ b: CGFloat) {
        context.setFillColor(CGColor(srgbRed: r, green: g, blue: b, alpha: 1))
        context.beginPath()
        context.addLines(between: points.map { CGPoint(x: $0.x, y: CGFloat(height) - $0.y) })
        context.closePath()
        context.fillPath()
    }
    /// A point of the panel by how far along each side, top left being (0, 0).
    func at(_ u: CGFloat, _ v: CGFloat) -> CGPoint {
        let top = CGPoint(
            x: corners[0].x + (corners[1].x - corners[0].x) * u,
            y: corners[0].y + (corners[1].y - corners[0].y) * u)
        let bottom = CGPoint(
            x: corners[3].x + (corners[2].x - corners[3].x) * u,
            y: corners[3].y + (corners[2].y - corners[3].y) * u)
        return CGPoint(x: top.x + (bottom.x - top.x) * v, y: top.y + (bottom.y - top.y) * v)
    }
    func patch(_ u: CGFloat, _ v: CGFloat, _ side: CGFloat = 0.13) -> [CGPoint] {
        [at(u, v), at(u + side, v), at(u + side, v + side), at(u, v + side)]
    }
    fill(corners, 0.85, 0.85, 0.85)
    fill(patch(0.05, 0.05), 0.85, 0.05, 0.05)
    fill(patch(0.82, 0.05), 0.05, 0.75, 0.05)
    fill(patch(0.82, 0.82), 0.05, 0.05, 0.85)
    fill(patch(0.2, 0.2, 0.6), 0.2, 0.2, 0.22)
    guard let image = context.makeImage() else { throw Failure("no image from the context") }
    return (image, corners)
}

/// What a sample of the rectified panel looks like, by its largest channel, or `grey` when none leads.
func hue(_ rgb: (Int, Int, Int)) -> String {
    let (r, g, b) = rgb
    if r > g + 60 && r > b + 60 { return "red" }
    if g > r + 60 && g > b + 60 { return "green" }
    if b > r + 60 && b > g + 60 { return "blue" }
    return "grey"
}

/// Call 7's two paths with no photo. A photo can show a wrong turn by eye but cannot show the corner
/// order is right everywhere, and if all six stills find a rectangle, nothing else runs the fallback.
func selftestRectify(directory: URL) throws -> [String] {
    var failures: [String] = []
    let (panel, corners) = try drawSkewedPanel()
    let panelPNG = directory.appendingPathComponent("panel.png")
    let panelOut = directory.appendingPathComponent("panel-rectified.png")
    try writeImage(panel, to: panelPNG, type: .png)
    _ = try rectifyJSON(input: panelPNG, output: panelOut)

    let (rectangles, _) = try detectRectangles(try loadCGImage(panelPNG))
    guard let rectangle = rectangles.first else {
        return ["rectify: no rectangle on the drawn panel"]
    }
    // 1% of the short side. Measured at 2.62 px on macOS 26.2, against 722 with the y flip left out and
    // 203 with the smaller panel kept, so the limit is far from both sides and needs no finer setting.
    let limit: CGFloat = 0.01 * CGFloat(selftestHeight)
    var worst: CGFloat = 0
    for (got, drawn) in zip(rectangle.corners, corners) {
        worst = max(worst, abs(got.x - drawn.x), abs(got.y - drawn.y))
    }
    print(
        "rectify of a drawn panel: kept the largest of \(rectangles.count), its worst corner "
            + "\(String(format: "%.2f", worst)) px from the drawing, and the limit is \(limit)")
    if rectangles.count < 2 { failures.append("rectify: the inner panel was not offered, so nothing chose") }
    if worst > limit { failures.append("rectify: a corner is \(worst) px from the drawing") }

    let pixels = try Pixels(try loadCGImage(panelOut))
    if pixels.width <= pixels.height {
        failures.append("rectify: the panel came out \(pixels.width)x\(pixels.height), and it is wider than tall")
    }
    let samples: [(String, Double, Double, String)] = [
        ("top left", 0.115, 0.115, "red"), ("top right", 0.885, 0.115, "green"),
        ("bottom right", 0.885, 0.885, "blue"), ("bottom left", 0.115, 0.885, "grey"),
    ]
    for (name, u, v, expected) in samples {
        let got = hue(pixels.rgb(Int(u * Double(pixels.width)), Int(v * Double(pixels.height))))
        if got != expected { failures.append("rectify: the \(name) is \(got), not \(expected)") }
    }

    guard let context = CGContext(
        data: nil, width: selftestWidth, height: selftestHeight, bitsPerComponent: 8, bytesPerRow: 0,
        space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw Failure("no bitmap context") }
    context.setFillColor(CGColor(srgbRed: 0.07, green: 0.08, blue: 0.11, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: selftestWidth, height: selftestHeight))
    guard let blank = context.makeImage() else { throw Failure("no image from the context") }
    let blankPNG = directory.appendingPathComponent("blank.png")
    let blankOut = directory.appendingPathComponent("blank-rectified.png")
    try writeImage(blank, to: blankPNG, type: .png)
    let answer = try JSONSerialization.jsonObject(
        with: Data(try rectifyJSON(input: blankPNG, output: blankOut).utf8)) as? [String: Any]
    if answer?["found"] as? Bool != false {
        failures.append("rectify: a blank frame found a rectangle")
    }
    if answer?["notes"] as? [String] != [uncroppedNote] {
        failures.append("rectify: a blank frame's notes are \(String(describing: answer?["notes"]))")
    }
    if try Data(contentsOf: blankOut) != Data(contentsOf: blankPNG) {
        failures.append("rectify: falling back did not leave the input's bytes")
    }
    return failures
}

func selftest(keep: URL?) throws -> Int32 {
    let directory =
        keep
        ?? FileManager.default.temporaryDirectory.appendingPathComponent(
            "vision-ocr-selftest-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer {
        if keep == nil { try? FileManager.default.removeItem(at: directory) }
    }

    print("selftest on macOS \(osString())")
    var failures: [String] = []
    let (upright, drawn) = try drawSelftestImage()
    failures += checkFixtureSeesFlips(drawn)

    // 1. The drawing is where it says, in the rows of the file, before Vision is asked anything.
    let drawnPNG = directory.appendingPathComponent("drawn.png")
    try writeImage(upright, to: drawnPNG, type: .png)
    let fromFile = try loadCGImage(drawnPNG)
    let filePixels = try Pixels(fromFile)
    for word in drawn {
        guard let ink = inkBounds(in: filePixels, around: word.rect) else {
            failures.append("drawing: no ink near '\(word.text)'")
            continue
        }
        // Two pixels: the glyph path's bounds against an antialiased edge, cut at half brightness.
        let edges = [
            ink.minX - word.rect.minX, ink.minY - word.rect.minY,
            ink.maxX - word.rect.maxX, ink.maxY - word.rect.maxY,
        ]
        if edges.contains(where: { abs($0) > 2 }) {
            failures.append("drawing: '\(word.text)' has its ink at \(ink), not \(word.rect)")
        }
    }

    // 2. Vision's boxes against those rectangles, through the one path a photo takes.
    print("read of the drawn PNG:")
    let direct = try recognize(fromFile)
    failures += compareWords(direct.boxes, drawn, label: "read", verbose: true)
    if direct.revision != VNRecognizeTextRequestRevision3 {
        failures.append("read: revision \(direct.revision), not 3")
    }
    let unasked = direct.boxes.filter { box in !drawn.contains { $0.text == box.text } }
    for box in unasked { print("  extra box, not a failure: '\(box.text)'") }

    // 3. decode leaves an sRGB PNG alone, and what comes out still reads in the same place.
    let decodedPNG = directory.appendingPathComponent("decoded.png")
    let size = try decode(input: drawnPNG, output: decodedPNG)
    if size.width != selftestWidth || size.height != selftestHeight {
        failures.append("decode: \(size.width)x\(size.height) from \(selftestWidth)x\(selftestHeight)")
    }
    let decoded = try loadCGImage(decodedPNG)
    if decoded.colorSpace?.name != CGColorSpace.sRGB {
        failures.append("decode: the PNG is tagged \(String(describing: decoded.colorSpace?.name)), not sRGB")
    }
    let decodedPixels = try Pixels(decoded)
    var worst = 0
    for y in 0..<decodedPixels.height {
        for x in 0..<decodedPixels.width {
            let a = filePixels.rgb(x, y)
            let b = decodedPixels.rgb(x, y)
            worst = max(worst, abs(a.0 - b.0), abs(a.1 - b.1), abs(a.2 - b.2))
        }
    }
    print("decode of an sRGB PNG: the largest change in any channel of any pixel is \(worst)")
    if worst > 1 { failures.append("decode: an sRGB PNG moved by \(worst) levels") }
    failures += compareWords(
        try recognize(decoded).boxes, drawn, label: "read after decode", verbose: false)

    // 4. Orientation is applied exactly once, for both containers a phone's capture can be. There is
    // no HEIC on this Mac yet (the decision of 2026-11-04), so this is the only HEIC decode proven.
    let stored = try storedForOrientation6(upright)
    for (name, type) in [("jpeg", UTType.jpeg), ("heic", UTType.heic)] {
        let tagged = directory.appendingPathComponent("orientation6.\(name)")
        try writeImage(
            stored, to: tagged, type: type,
            properties: [
                kCGImagePropertyOrientation: 6,
                kCGImageDestinationLossyCompressionQuality: 1.0,
            ])
        let output = directory.appendingPathComponent("orientation6-\(name).png")
        let turned = try decode(input: tagged, output: output)
        if turned.width != selftestWidth || turned.height != selftestHeight {
            failures.append("decode \(name): \(turned.width)x\(turned.height), the turn was not applied")
        }
        let tag = imageProperties(output)[kCGImagePropertyOrientation as String]
        if let tag = tag as? Int, tag != 1 {
            failures.append("decode \(name): the PNG still says Orientation \(tag)")
        }
        failures += compareWords(
            try recognize(try loadCGImage(output)).boxes, drawn,
            label: "read after decode of Orientation 6 \(name)", verbose: false)
    }

    // 5. Colour. Yellow is the case that matters: Full Swing lights a sign letter yellow, and
    // `LIT_BLUE_RATIO` reads the blue of that letter. Grey must not move, since the two spaces share a
    // white point and a curve. A level or two either way is Core Image's rounding.
    let patches: [(String, (UInt8, UInt8, UInt8), CGColorSpace, [Double])] = [
        ("Display P3 yellow", (235, 200, 70), displayP3, p3ToSRGB((235, 200, 70))),
        ("Display P3 grey", (128, 128, 128), displayP3, p3ToSRGB((128, 128, 128))),
        ("sRGB yellow", (235, 200, 70), sRGB, [235, 200, 70]),
    ]
    for (index, patch) in patches.enumerated() {
        let input = directory.appendingPathComponent("patch\(index).png")
        let output = directory.appendingPathComponent("patch\(index)-decoded.png")
        try writeImage(try solidImage(patch.1, space: patch.2), to: input, type: .png)
        _ = try decode(input: input, output: output)
        let (r, g, b) = try Pixels(try loadCGImage(output)).rgb(8, 8)
        let expected = patch.3
        print(
            "decode of \(patch.0) \(patch.1): got (\(r), \(g), \(b)), worked by hand "
                + String(format: "(%.1f, %.1f, %.1f)", expected[0], expected[1], expected[2]))
        let got = [Double(r), Double(g), Double(b)]
        if zip(got, expected).contains(where: { abs($0 - $1) > 2 }) {
            failures.append("decode: \(patch.0) came out (\(r), \(g), \(b))")
        }
    }

    // 6. The rectifier's two paths.
    failures += try selftestRectify(directory: directory)

    if failures.isEmpty {
        let held = drawn.filter { $0.held }.count
        print(
            "selftest: passed, \(drawn.count) words (\(held) held to the limit) on each of the 4 reads, "
                + "\(patches.count) colour patches, and the rectifier's found and fallback paths")
        return 0
    }
    for failure in failures { print("FAIL \(failure)") }
    print("selftest: \(failures.count) failed")
    return 1
}

// MARK: - The commands

let usage = """
    usage: vision-ocr decode IN OUT.png
           vision-ocr read IMAGE [--config C0]
           vision-ocr glyph IMAGE BOXES.json
           vision-ocr rectify IN.png OUT.png
           vision-ocr selftest [DIR]
    """

func run(_ arguments: [String]) throws -> Int32 {
    switch arguments.first {
    case "decode" where arguments.count == 3:
        let size = try decode(
            input: URL(fileURLWithPath: arguments[1]), output: URL(fileURLWithPath: arguments[2]))
        writeStdout(
            "{\"image\": \(try jsonString(arguments[2])), \"width\": \(size.width), "
                + "\"height\": \(size.height), \"os\": \(try jsonString(osString()))}")
        return 0
    case "read" where arguments.count == 2:
        writeStdout(try readJSON(path: arguments[1], config: "C0"))
        return 0
    case "read" where arguments.count == 4 && arguments[2] == "--config":
        // The ladder's other rungs are P12's. Refusing them now is better than printing C0's boxes
        // labelled as something else.
        guard arguments[3] == "C0" else { throw Failure("config \(arguments[3]) is not built yet") }
        writeStdout(try readJSON(path: arguments[1], config: "C0"))
        return 0
    case "glyph" where arguments.count == 3:
        writeStdout(try glyphJSON(image: arguments[1], boxes: arguments[2]))
        return 0
    case "rectify" where arguments.count == 3:
        writeStdout(
            try rectifyJSON(
                input: URL(fileURLWithPath: arguments[1]), output: URL(fileURLWithPath: arguments[2])))
        return 0
    case "selftest" where arguments.count <= 2:
        return try selftest(keep: arguments.count == 2 ? URL(fileURLWithPath: arguments[1]) : nil)
    default:
        FileHandle.standardError.write(Data((usage + "\n").utf8))
        return 2
    }
}

do {
    exit(try run(Array(CommandLine.arguments.dropFirst())))
} catch {
    FileHandle.standardError.write(Data("vision-ocr: \(error)\n".utf8))
    exit(1)
}
