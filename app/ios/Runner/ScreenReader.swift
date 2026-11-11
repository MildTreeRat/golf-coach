// ScreenReader.swift: one photo of the launch monitor's screen to Vision's boxes, as the JSON text
// `crates/app`'s `record_photo` takes. [M38 P17]
//
// This is M33's C0 reader as code that ships (the plan's C5): the settings, the decode, the y flip
// and `TextBox::rgb`'s definition are `spikes/vision-ocr/vision_ocr.swift`'s, copied rather than
// re-derived, because M33's committed boxes are the measurement and a re-derivation would be a second
// reader with nothing holding it to the first. `app/tool/reader_check.sh` is what holds it: it
// compiles this file for macOS and requires all six captures to give M33's boxes to the bit.
//
// Foundation, CoreImage, ImageIO, Vision and CoreGraphics only, and no UIKit or Flutter (C6), so that
// the file the app ships is the file that check compiles, and not a copy of it.
//
// One thing differs from the spike on purpose: the decode stays in memory. The spike wrote the upright
// image to a PNG and read the PNG back, and the app hands the rendered image straight to Vision.
// P17's check found the two give the same boxes on all six captures (the plan's findings), which is
// the condition C5 set for keeping it this way.

import CoreGraphics
import CoreImage
import Foundation
import ImageIO
import Vision

/// A namespace and not a type to make, so that the helpers below are `ScreenReader.Pixels` and so on,
/// and cannot meet a name in the Runner's other Swift files.
enum ScreenReader {
    struct Failure: Error, CustomStringConvertible {
        let description: String
        init(_ description: String) { self.description = description }
    }

    /// The reading settings' name in M33's ladder, written into every answer so that a box file says
    /// which reader made it. Only C0 is built.
    static let config = "C0"

    /// Reads the photo at `path` and answers the boxes as JSON text, and how long the whole read took.
    ///
    /// The text is M33's `read` output, key for key and one box to a line, so a phone's box file diffs
    /// against a committed one by the box: `image` (the path given), `width` and `height` (the upright
    /// image the boxes are in), `config`, `revision`, `os`, `platform`, and `boxes` in
    /// `screen::TextBox`'s keys. `platform` is the one key the spike has no need of, because on the
    /// Simulator `os` alone does not say which system read the photo (P18's finding says what it holds
    /// there).
    ///
    /// `msVision` is the whole call, the decode and `rgb` with Vision between them, and not Vision's
    /// `perform` alone: it is the Swift side's share of `ms_total`, beside Rust's `ms_rust`, so that the
    /// bay's three timings account for one another and a slow decode cannot hide between them.
    ///
    /// It throws for a photo that cannot be opened or rendered, and answers an empty `boxes` for one
    /// that opens and holds no text, since that is a read of nothing and Rust answers it as `Failed`.
    static func read(path: String) throws -> (json: String, msVision: Int) {
        let started = DispatchTime.now().uptimeNanoseconds
        let image = try decode(URL(fileURLWithPath: path))
        var (boxes, revision) = try recognize(image)
        let pixels = try Pixels(image)
        for index in boxes.indices {
            let box = boxes[index]
            boxes[index].rgb = glyphRGB(
                pixels, x: box.x, y: box.y, width: box.width, height: box.height)
        }
        let json = try answer(
            path: path, width: image.width, height: image.height, revision: revision, boxes: boxes)
        let elapsed = DispatchTime.now().uptimeNanoseconds - started
        return (json, Int(elapsed / 1_000_000))
    }

    // MARK: - Images

    static let sRGB = CGColorSpace(name: CGColorSpace.sRGB)!
    /// One for the process, since a context is costly to make and safe to share across threads.
    static let ciContext = CIContext()

    /// The spike's `decode`, kept in memory: ImageIO through Core Image, with the file's Orientation
    /// applied once and the colours matched to 8-bit sRGB. So a phone's portrait, stored turned with a
    /// tag that says so, reaches Vision upright, and a Display P3 HEIC reaches `rgb` in sRGB, which is
    /// the space `LIT_BLUE_RATIO` was measured in.
    static func decode(_ url: URL) throws -> CGImage {
        guard let image = CIImage(contentsOf: url, options: [.applyOrientationProperty: true]) else {
            throw Failure("Core Image cannot open \(url.path)")
        }
        guard
            let rendered = ciContext.createCGImage(
                image, from: image.extent, format: .RGBA8, colorSpace: sRGB)
        else { throw Failure("Core Image cannot render \(url.path)") }
        return rendered
    }

    /// The bytes of the decoded image as they are, with no second colour match: M33's log, "For M38"
    /// item 3. Read through a `CGContext.draw` into sRGB instead, 489 of 500 boxes on a Display P3 PNG
    /// moved off Rust's answer in M33, and this moves none. The stride is read off the image, and a
    /// layout the scans would misread is refused rather than read as RGB.
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
            // Little-endian would put blue first with nothing else about the image looking wrong, and
            // a lit letter is told from an unlit one by its blue.
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

    // MARK: - Vision, at C0

    struct Box {
        let text: String
        let x: Double
        let y: Double
        let width: Double
        let height: Double
        let confidence: Double
        /// `TextBox::rgb`, filled after the mapping by `glyphRGB` in the image the coordinates are in.
        /// `nil` is both "not measured yet" and "holds no pixel", as `null` is in the JSON.
        var rgb: [Double]? = nil
    }

    /// M33's C0, which is the only configuration it measured. Custom words are empty because Apple's
    /// reference says they are ignored with language correction off, so setting them would claim
    /// something C0 does not do.
    ///
    /// The revision is set first because it decides which languages the request can take, and pinned
    /// rather than left at the default so that a later iOS cannot change the recognizer under the bars.
    /// It and `automaticallyDetectsLanguage` are why the app's deployment target is iOS 16 (D2).
    static func recognize(_ image: CGImage) throws -> (boxes: [Box], revision: Int) {
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
            // so y is the flip of the box's far edge. Unrounded, because `screen`'s tie rule reads
            // positions finer than a pixel.
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

    // MARK: - `TextBox::rgb`

    /// One axis of a box clipped to an image `extent` pixels long, as `[start, end)` pixel indices, or
    /// `nil` when that is empty: `crates/ocr/src/recognizer.rs::clipped`, line for line. So a box with
    /// no width at a fractional x still holds one column, because the floor and the ceil differ, and
    /// that is the Rust's behaviour rather than something to tidy here.
    static func clipped(_ start: Double, _ length: Double, _ extent: Int) -> (low: Int, high: Int)? {
        let low = max(start, 0).rounded(.down)
        let high = min((start + length).rounded(.up), Double(extent))
        if low >= high { return nil }
        return (Int(low), Int(high))
    }

    /// `crates/ocr/src/recognizer.rs::glyph_rgb`, which is `TextBox::rgb`'s definition: the mean of the
    /// brightest quarter of the box's pixels, the ranks from `n * 3 / 4` to the end, by `r + g + b`
    /// ascending with ties in row-major order.
    ///
    /// Rust gets its tie rule from a stable sort. Swift's `sort` is not documented as stable, so the
    /// tie rule is in the key instead: the sum above the pixel's row-major index, which makes every key
    /// distinct and the order the same whatever the sort does. "The brightest quarter" with any other
    /// tie rule does not reproduce M33's numbers to the bit (its log, "For M38" item 3). A box holds at
    /// most the image's pixels, so the index fits below bit 40 and the sum, at most 765, above it.
    ///
    /// The running totals are Doubles of integers below 2^53, as in Rust, so the division is the only
    /// rounding and both languages do the same one.
    static func glyphRGB(_ pixels: Pixels, x: Double, y: Double, width: Double, height: Double)
        -> [Double]?
    {
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

    // MARK: - JSON, written by hand

    /// By hand rather than `JSONEncoder`, for the keys in M33's order and one box to a line. A Swift
    /// Double's `description` is the shortest text that round-trips, which is what lets the check
    /// compare with `==` after the JSON trip, and Rust read back the bits Vision gave.
    ///
    /// It throws where the spike's precondition stopped, because in an app that is a failed read with
    /// a sentence and not a crash. No finite box gives a number that is not.
    static func jsonNumber(_ value: Double) throws -> String {
        guard value.isFinite else {
            throw Failure("a box number that is not finite has no JSON spelling: \(value)")
        }
        return value.description
    }

    static func jsonString(_ value: String) throws -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .withoutEscapingSlashes
        return String(decoding: try encoder.encode(value), as: UTF8.self)
    }

    static func jsonRGB(_ rgb: [Double]?) throws -> String {
        guard let rgb else { return "null" }
        return "[" + (try rgb.map(jsonNumber)).joined(separator: ", ") + "]"
    }

    static func answer(path: String, width: Int, height: Int, revision: Int, boxes: [Box]) throws
        -> String
    {
        var lines: [String] = []
        lines.append("{")
        lines.append("  \"image\": \(try jsonString(path)),")
        lines.append("  \"width\": \(width),")
        lines.append("  \"height\": \(height),")
        lines.append("  \"config\": \(try jsonString(config)),")
        lines.append("  \"revision\": \(revision),")
        lines.append("  \"os\": \(try jsonString(osString())),")
        lines.append("  \"platform\": \(try jsonString(platform)),")
        if boxes.isEmpty {
            lines.append("  \"boxes\": []")
        } else {
            lines.append("  \"boxes\": [")
            for (index, box) in boxes.enumerated() {
                let comma = index + 1 < boxes.count ? "," : ""
                let row =
                    "{\"text\": \(try jsonString(box.text)), \"x\": \(try jsonNumber(box.x)), "
                    + "\"y\": \(try jsonNumber(box.y)), \"width\": \(try jsonNumber(box.width)), "
                    + "\"height\": \(try jsonNumber(box.height)), "
                    + "\"confidence\": \(try jsonNumber(box.confidence)), "
                    + "\"rgb\": \(try jsonRGB(box.rgb))}"
                lines.append("    \(row)\(comma)")
            }
            lines.append("  ]")
        }
        lines.append("}")
        return lines.joined(separator: "\n")
    }

    // MARK: - Where the read was made

    /// The spike's spelling, `26.2 (25C56)`, so a phone's file and M33's compare by eye: the system's
    /// version and the kernel's build.
    ///
    /// The build is decoded up to its terminating zero rather than through the spike's
    /// `String(cString:)` on an array, which Swift 6 deprecates. Same text, and no warning waiting for
    /// the Runner's move off Swift 5.
    static func osString() -> String {
        let v = ProcessInfo.processInfo.operatingSystemVersion
        var size = 0
        sysctlbyname("kern.osversion", nil, &size, nil, 0)
        var buffer = [UInt8](repeating: 0, count: max(size, 1))
        sysctlbyname("kern.osversion", &buffer, &size, nil, 0)
        let build = String(decoding: buffer.prefix { $0 != 0 }, as: UTF8.self)
        let version =
            v.patchVersion == 0
            ? "\(v.majorVersion).\(v.minorVersion)"
            : "\(v.majorVersion).\(v.minorVersion).\(v.patchVersion)"
        return "\(version) (\(build))"
    }

    /// Decided when the file is compiled, since it is a fact about the build: the check's macOS binary,
    /// the Simulator's app, or a device's.
    static let platform: String = {
        #if os(macOS)
            return "macOS"
        #elseif targetEnvironment(simulator)
            return "iOS Simulator"
        #else
            return "iOS"
        #endif
    }()
}
