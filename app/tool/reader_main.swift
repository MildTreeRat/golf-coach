// reader_main.swift: holds the app's `ScreenReader.swift` to M33's committed boxes, and its
// `PhotoTime.swift` to C9, on macOS. [M38 P17, P20]
//
//   app/tool/reader_check.sh          (builds this beside the app's files, and runs it)
//
// Compiled with `app/ios/Runner/ScreenReader.swift` and `PhotoTime.swift` themselves, and never copies,
// so what passes here is the files the app ships (C6). Three checks:
//
//   1. Every capture M33 committed boxes for, read by the app's reader, against
//      `spikes/vision-ocr/boxes/C0/raw/<id>.json`: the same count, and each box's text and every number
//      equal by `==` after the JSON trip. M33 measured on this Mac's macOS, so a difference is the
//      port's and not the system's, and the check prints both systems so that a later run on another
//      macOS says why it may differ.
//   2. A drawn JPEG stored turned, with Orientation 6 saying so: the reader's image comes out with the
//      width and height swapped back, and two words land where they were drawn upright. A control
//      without the tag must come out unturned, or the first half of that would prove nothing.
//   3. `PhotoTime` on JPEGs written here, one per route of C9: an EXIF date with its offset, a date
//      alone (in summer and in winter, so the device's offset is the one at that date), and neither,
//      which falls to the file's date, and then to the clock for a file that is not there. The device's
//      zone and the clock are fixed, so the answers are exact strings.
//
// Everything read off the captures is labelled `provisional: pasted copies` (rule 12): they are
// chat-pasted JPEGs, not the photos as taken.

import CoreGraphics
import CoreText
import CryptoKit
import Foundation
import ImageIO
import UniformTypeIdentifiers

/// A box file in the keys both the reader and M33 write. Keys beyond these, such as the reader's
/// `image` and `platform`, are not compared: the first names a path and the second is the app's alone.
struct BoxFile: Decodable {
    struct Box: Decodable {
        let text: String
        let x: Double
        let y: Double
        let width: Double
        let height: Double
        let confidence: Double
        let rgb: [Double]?
    }
    let width: Int
    let height: Int
    let config: String
    let revision: Int
    let os: String
    let boxes: [Box]
}

struct Manifest: Decodable {
    struct Capture: Decodable {
        let id: String
        let path: String
        let sha256: String
    }
    let captures: [Capture]
}

/// Which of a box's fields differ, by name, so a failure says what moved and not only that one did.
func differences(_ got: BoxFile.Box, _ want: BoxFile.Box) -> [String] {
    var moved: [String] = []
    if got.text != want.text { moved.append("text \(got.text) for \(want.text)") }
    let numbers: [(String, Double, Double)] = [
        ("x", got.x, want.x), ("y", got.y, want.y), ("width", got.width, want.width),
        ("height", got.height, want.height), ("confidence", got.confidence, want.confidence),
    ]
    for (name, a, b) in numbers where a != b { moved.append("\(name) \(a) for \(b)") }
    if got.rgb != want.rgb {
        moved.append("rgb \(String(describing: got.rgb)) for \(String(describing: want.rgb))")
    }
    return moved
}

/// The box rows of a file as text. Not what the plan's gate is, which is the numbers by `==`, but a
/// stronger thing when it holds: the same bytes, so nothing about the spelling moved either.
func boxRows(_ text: String) -> [String] {
    text.split(separator: "\n").map(String.init).filter { $0.hasPrefix("    {\"text\": ") }
}

func checkCaptures(repo: URL) throws -> [String] {
    let spike = repo.appendingPathComponent("spikes/vision-ocr")
    let committedDir = spike.appendingPathComponent("boxes/C0/raw")
    let manifest = try JSONDecoder().decode(
        Manifest.self, from: Data(contentsOf: spike.appendingPathComponent("photos.json")))
    let ids = try FileManager.default.contentsOfDirectory(atPath: committedDir.path)
        .filter { $0.hasSuffix(".json") }
        .map { String($0.dropLast(".json".count)) }
        .sorted()
    if ids.isEmpty { return ["captures: no committed box file under \(committedDir.path)"] }

    print("The captures against M33's committed boxes (provisional: pasted copies), at C0:")
    print("  capture          boxes  differ  rows same  ms (this Mac)")
    var failures: [String] = []
    var systems: Set<String> = []
    for id in ids {
        guard let capture = manifest.captures.first(where: { $0.id == id }) else {
            failures.append("\(id): committed boxes with no capture of that id in photos.json")
            continue
        }
        let photo = repo.appendingPathComponent(capture.path)
        let digest = SHA256.hash(data: try Data(contentsOf: photo))
            .map { String(format: "%02x", $0) }.joined()
        if digest != capture.sha256 {
            failures.append("\(id): \(capture.path) is not the capture M33 measured (sha256 \(digest))")
            continue
        }

        let committedText = try String(
            contentsOf: committedDir.appendingPathComponent("\(id).json"), encoding: .utf8)
        let want = try JSONDecoder().decode(BoxFile.self, from: Data(committedText.utf8))
        let (json, ms) = try ScreenReader.read(path: photo.path)
        let got = try JSONDecoder().decode(BoxFile.self, from: Data(json.utf8))
        systems.insert("this Mac \(got.os), M33 \(want.os)")

        let headers: [(String, String, String)] = [
            ("width", "\(got.width)", "\(want.width)"), ("height", "\(got.height)", "\(want.height)"),
            ("config", got.config, want.config), ("revision", "\(got.revision)", "\(want.revision)"),
        ]
        for (name, a, b) in headers where a != b { failures.append("\(id): \(name) \(a) for \(b)") }
        if got.boxes.count != want.boxes.count {
            failures.append("\(id): \(got.boxes.count) boxes for M33's \(want.boxes.count)")
        }
        var differ = 0
        for (index, (a, b)) in zip(got.boxes, want.boxes).enumerated() {
            let moved = differences(a, b)
            if moved.isEmpty { continue }
            differ += 1
            failures.append("\(id) box \(index): " + moved.joined(separator: "; "))
        }
        let rowsSame = boxRows(json) == boxRows(committedText)
        let name = id.padding(toLength: 15, withPad: " ", startingAt: 0)
        print(
            "  \(name) \(String(format: "%6d  %6d", got.boxes.count, differ))  "
                + "\(rowsSame ? "yes" : "no ")        \(ms)")
    }
    for system in systems.sorted() { print("  systems: \(system)") }
    return failures
}

// MARK: - Orientation

let sRGB = CGColorSpace(name: CGColorSpace.sRGB)!

/// Where the fixture draws: off-centre on both axes and not square, so a missing turn swaps the frame's
/// sides, and a turn or a flip the wrong way moves each word far from where it was drawn.
let frameWidth = 1600
let frameHeight = 1000

struct Word {
    let text: String
    let size: CGFloat
    /// The top left of the word's ink, in pixels from the top left of the upright image.
    let x: CGFloat
    let y: CGFloat
}

/// Two of the spike's self-test words at its sizes and places, the two in opposite corners. Both sat
/// inside the limit below on its Orientation 6 read, on this macOS.
let words = [
    Word(text: "CARRY", size: 64, x: 120, y: 70),
    Word(text: "TOTAL", size: 64, x: 1240, y: 880),
]

/// The spike's limit: 5% of the drawn word's height, held on the box's centre, because Vision's box is
/// not the ink's rectangle and no edge can hold it. A quarter turn moves either word by hundreds of
/// pixels, so the limit needs no finer setting here.
let tolerance: CGFloat = 0.05

/// White words on near-black, upright, and each word's ink rectangle from Core Text's glyph paths.
func drawUpright() throws -> (CGImage, [(String, CGRect)]) {
    guard
        let context = CGContext(
            data: nil, width: frameWidth, height: frameHeight, bitsPerComponent: 8, bytesPerRow: 0,
            space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw ScreenReader.Failure("no bitmap context") }
    context.setFillColor(CGColor(srgbRed: 18.0 / 255, green: 20.0 / 255, blue: 28.0 / 255, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: frameWidth, height: frameHeight))
    var drawn: [(String, CGRect)] = []
    for word in words {
        let font = CTFontCreateWithName("Helvetica-Bold" as CFString, word.size, nil)
        let attributes: [NSAttributedString.Key: Any] = [
            NSAttributedString.Key(kCTFontAttributeName as String): font,
            NSAttributedString.Key(kCTForegroundColorAttributeName as String):
                CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 1),
        ]
        let line = CTLineCreateWithAttributedString(
            NSAttributedString(string: word.text, attributes: attributes))
        // Relative to the baseline's origin, y up, where the image's rows run down.
        let ink = CTLineGetBoundsWithOptions(line, .useGlyphPathBounds)
        context.textPosition = CGPoint(x: word.x - ink.minX, y: CGFloat(frameHeight) - word.y - ink.maxY)
        CTLineDraw(line, context)
        drawn.append((word.text, CGRect(x: word.x, y: word.y, width: ink.width, height: ink.height)))
    }
    guard let image = context.makeImage() else { throw ScreenReader.Failure("no image") }
    return (image, drawn)
}

/// The spike's `storedForOrientation6`: the stored image of an upright picture whose file says
/// Orientation 6, turned 90 degrees anticlockwise so that a reader honouring the tag turns it back.
/// This is how a phone stores an upright portrait.
func storedForOrientation6(_ upright: CGImage) throws -> CGImage {
    let width = upright.width
    let height = upright.height
    guard
        let context = CGContext(
            data: nil, width: height, height: width, bitsPerComponent: 8, bytesPerRow: 0,
            space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw ScreenReader.Failure("no bitmap context") }
    context.translateBy(x: CGFloat(height), y: 0)
    context.rotate(by: .pi / 2)
    context.draw(upright, in: CGRect(x: 0, y: 0, width: width, height: height))
    guard let image = context.makeImage() else { throw ScreenReader.Failure("no image") }
    return image
}

func writeJPEG(_ image: CGImage, to url: URL, orientation: Int?) throws {
    guard
        let destination = CGImageDestinationCreateWithURL(
            url as CFURL, UTType.jpeg.identifier as CFString, 1, nil)
    else { throw ScreenReader.Failure("this Mac cannot write a JPEG to \(url.path)") }
    var properties: [CFString: Any] = [kCGImageDestinationLossyCompressionQuality: 1.0]
    if let orientation { properties[kCGImagePropertyOrientation] = orientation }
    CGImageDestinationAddImage(destination, image, properties as CFDictionary)
    guard CGImageDestinationFinalize(destination) else {
        throw ScreenReader.Failure("writing \(url.path) failed")
    }
}

func checkOrientation(scratch: URL) throws -> [String] {
    try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
    let (upright, drawn) = try drawUpright()
    let stored = try storedForOrientation6(upright)
    let tagged = scratch.appendingPathComponent("orientation6.jpg")
    let untagged = scratch.appendingPathComponent("untagged.jpg")
    try writeJPEG(stored, to: tagged, orientation: 6)
    try writeJPEG(stored, to: untagged, orientation: nil)

    var failures: [String] = []
    let turned = try JSONDecoder().decode(
        BoxFile.self, from: Data(try ScreenReader.read(path: tagged.path).json.utf8))
    print(
        "Orientation 6: stored \(stored.width)x\(stored.height), read as \(turned.width)x\(turned.height)"
            + ", drawn upright as \(frameWidth)x\(frameHeight)")
    if turned.width != frameWidth || turned.height != frameHeight {
        failures.append("orientation: the tagged JPEG read as \(turned.width)x\(turned.height)")
    }
    for (text, rect) in drawn {
        guard let box = turned.boxes.first(where: { $0.text == text }) else {
            let found = turned.boxes.map { "'\($0.text)'" }.joined(separator: " ")
            failures.append("orientation: '\(text)' did not come back upright; Vision read \(found)")
            continue
        }
        let dx = CGFloat(box.x + box.width / 2) - rect.midX
        let dy = CGFloat(box.y + box.height / 2) - rect.midY
        let limit = tolerance * rect.height
        print(
            "  '\(text)': centre off by "
                + String(format: "(%.2f, %.2f) px, limit %.2f", dx, dy, limit))
        if abs(dx) > limit || abs(dy) > limit {
            failures.append("orientation: '\(text)' landed off by (\(dx), \(dy)), limit \(limit)")
        }
    }

    // The control: the same bytes with no tag. If this came out turned too, the size check above would
    // pass on a reader that turns everything, and so prove nothing about the tag.
    let plain = try JSONDecoder().decode(
        BoxFile.self, from: Data(try ScreenReader.read(path: untagged.path).json.utf8))
    print("  control, untagged: read as \(plain.width)x\(plain.height)")
    if plain.width != stored.width || plain.height != stored.height {
        failures.append("orientation: the untagged control read as \(plain.width)x\(plain.height)")
    }
    return failures
}

// MARK: - A photo's time

/// A zone with summer time, which is what lets a date alone show whose offset it took: New York's is
/// -04:00 in July and -05:00 in November, and the offset written in the photo is neither.
let deviceZone = TimeZone(identifier: "America/New_York")!

/// One case of C9: what the JPEG's EXIF holds, the file's date, and the answer and route expected.
struct TimeCase {
    let name: String
    /// `DateTimeOriginal`, `OffsetTimeOriginal` and `SubSecTimeOriginal`, each written only if given.
    let original: String?
    let offset: String?
    let subsec: String?
    let fileDate: Date
    let want: String
    let route: PhotoTime.Route
}

/// 2026-03-01 17:34:56.789 UTC, a fraction on purpose, since C9's answer is in whole seconds.
let fileDate = Date(timeIntervalSince1970: 1_772_386_496.789)
/// The clock, for the one case that reaches it: 2026-11-05 19:12:03.456 UTC.
let clock = Date(timeIntervalSince1970: 1_793_905_923.456)

let timeCases = [
    TimeCase(
        name: "EXIF date and offset", original: "2026:07:14 18:05:09", offset: "+09:30",
        subsec: "731", fileDate: fileDate, want: "2026-07-14T18:05:09+09:30",
        route: .exifWithOffset),
    TimeCase(
        name: "EXIF date only, summer", original: "2026:07:14 18:05:09", offset: nil, subsec: nil,
        fileDate: fileDate, want: "2026-07-14T18:05:09-04:00", route: .exifInDeviceOffset),
    TimeCase(
        name: "EXIF date only, winter", original: "2026:11:05 18:05:09", offset: nil, subsec: nil,
        fileDate: fileDate, want: "2026-11-05T18:05:09-05:00", route: .exifInDeviceOffset),
    // A camera that knows no offset may write EXIF's blanks for one, and they are no offset.
    TimeCase(
        name: "EXIF date, blank offset", original: "2026:11:05 18:05:09", offset: "   :  ",
        subsec: nil, fileDate: fileDate, want: "2026-11-05T18:05:09-05:00",
        route: .exifInDeviceOffset),
    TimeCase(
        name: "no EXIF date", original: nil, offset: nil, subsec: nil, fileDate: fileDate,
        want: "2026-03-01T12:34:56-05:00", route: .fileDate),
    // A date that does not exist is refused rather than rolled into March, and the file's date
    // answers. A blank date is held below and not here, because ImageIO will not write one.
    TimeCase(
        name: "EXIF date impossible", original: "2026:02:30 10:00:00", offset: "-05:00",
        subsec: nil, fileDate: fileDate, want: "2026-03-01T12:34:56-05:00", route: .fileDate),
]

/// A small grey JPEG with the case's EXIF, and the case's file date as its modification time.
func writeTimed(_ timeCase: TimeCase, to url: URL) throws {
    guard
        let context = CGContext(
            data: nil, width: 64, height: 48, bitsPerComponent: 8, bytesPerRow: 0, space: sRGB,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { throw ScreenReader.Failure("no bitmap context") }
    context.setFillColor(CGColor(srgbRed: 0.5, green: 0.5, blue: 0.5, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: 64, height: 48))
    guard let image = context.makeImage(),
        let destination = CGImageDestinationCreateWithURL(
            url as CFURL, UTType.jpeg.identifier as CFString, 1, nil)
    else { throw ScreenReader.Failure("this Mac cannot write a JPEG to \(url.path)") }
    var exif: [CFString: Any] = [:]
    if let original = timeCase.original { exif[kCGImagePropertyExifDateTimeOriginal] = original }
    if let offset = timeCase.offset { exif[kCGImagePropertyExifOffsetTimeOriginal] = offset }
    if let subsec = timeCase.subsec { exif[kCGImagePropertyExifSubsecTimeOriginal] = subsec }
    var properties: [CFString: Any] = [kCGImageDestinationLossyCompressionQuality: 0.9]
    if !exif.isEmpty { properties[kCGImagePropertyExifDictionary] = exif }
    CGImageDestinationAddImage(destination, image, properties as CFDictionary)
    guard CGImageDestinationFinalize(destination) else {
        throw ScreenReader.Failure("writing \(url.path) failed")
    }
    try FileManager.default.setAttributes(
        [.modificationDate: timeCase.fileDate], ofItemAtPath: url.path)
}

func checkPhotoTime(scratch: URL) throws -> [String] {
    let directory = scratch.appendingPathComponent("photo-time")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    print("PhotoTime, with the device in \(deviceZone.identifier):")
    var failures: [String] = []
    func held(_ name: String, _ got: (text: String, route: PhotoTime.Route), _ want: String,
        _ route: PhotoTime.Route) {
        let padded = name.padding(toLength: 24, withPad: " ", startingAt: 0)
        print("  \(padded) \(got.text)  (\(got.route.rawValue))")
        if got.text != want || got.route != route {
            failures.append(
                "photo time, \(name): \(got.text) by \(got.route.rawValue), "
                    + "for \(want) by \(route.rawValue)")
        }
    }

    for (index, timeCase) in timeCases.enumerated() {
        let url = directory.appendingPathComponent("case-\(index + 1).jpg")
        try writeTimed(timeCase, to: url)
        // The case is only a case of its route if ImageIO wrote what it was given: a writer that dropped
        // the offset would turn the first case into the second, and both would still pass.
        let written = PhotoTime.exifOriginal(url)
        if written?.0 != timeCase.original || written?.1 != timeCase.offset {
            failures.append(
                "photo time, \(timeCase.name): the JPEG holds \(String(describing: written)), not "
                    + "what was written")
        }
        held(
            timeCase.name, PhotoTime.capturedAt(url: url, timeZone: deviceZone, now: clock),
            timeCase.want, timeCase.route)
    }
    held(
        "no file", PhotoTime.capturedAt(
            url: directory.appendingPathComponent("absent.jpg"), timeZone: deviceZone, now: clock),
        "2026-11-05T14:12:03-05:00", .clock)

    // What ImageIO will not write, held on the parsers themselves: a camera that knows no date writes
    // blanks in its shape, and an offset is `±HH:MM` in ASCII digits or it is none.
    for text in ["    :  :     :  :  ", "2026:11:05 18:05", "2026-11-05 18:05:09", "2026:11:05T18:05:09"]
    where PhotoTime.exifDate(text, in: deviceZone) != nil {
        failures.append("photo time: '\(text)' was read as an EXIF date")
    }
    for text in ["", "   :  ", "Z", "+5:00", "++5:00", "-05:60", "+19:00", "+05:00:00", "+٠٥:٠٠"]
    where PhotoTime.zone(forOffset: text) != nil {
        failures.append("photo time: '\(text)' was read as an offset")
    }
    if PhotoTime.zone(forOffset: "-03:30")?.secondsFromGMT() != -12_600 {
        failures.append("photo time: -03:30 is not 3.5 hours behind")
    }

    // The camera's: now, in the device's offset, whole seconds, and `+00:00` and not `Z` for UTC.
    let camera = PhotoTime.now(at: clock, in: deviceZone)
    let utc = PhotoTime.now(at: clock, in: TimeZone(identifier: "UTC")!)
    print("  camera                   \(camera), and in UTC \(utc)")
    if camera != "2026-11-05T14:12:03-05:00" { failures.append("photo time, camera: \(camera)") }
    if utc != "2026-11-05T19:12:03+00:00" { failures.append("photo time, camera in UTC: \(utc)") }
    return failures
}

@main
struct ReaderCheck {
    static func main() {
        let arguments = CommandLine.arguments.dropFirst()
        guard arguments.count == 2, let repoPath = arguments.first, let scratchPath = arguments.last
        else {
            FileHandle.standardError.write(Data("usage: reader-check REPO SCRATCH\n".utf8))
            exit(2)
        }
        do {
            var failures = try checkCaptures(repo: URL(fileURLWithPath: repoPath))
            failures += try checkOrientation(scratch: URL(fileURLWithPath: scratchPath))
            failures += try checkPhotoTime(scratch: URL(fileURLWithPath: scratchPath))
            if failures.isEmpty {
                print("reader_check: passed (the captures: provisional: pasted copies)")
                exit(0)
            }
            for failure in failures { print("FAIL \(failure)") }
            print("reader_check: \(failures.count) failed")
            exit(1)
        } catch {
            FileHandle.standardError.write(Data("reader-check: \(error)\n".utf8))
            exit(1)
        }
    }
}
