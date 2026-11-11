// PhotoTime.swift: when a photo was taken, as the RFC 3339 text with an offset that `record_photo`
// takes as `captured_at`. [M38 P20]
//
// The plan's C9, in its order. A photo from the camera is now, in the device's offset, since it is
// being taken. A photo from the library is its EXIF `DateTimeOriginal` with `OffsetTimeOriginal`;
// else that date in the device's offset at that date; else the file's modification date; else now.
//
// The answer stamps the shot, names the session (C8: the date in the photo's own offset, so a wrong
// offset moves an evening's shot into the next day's session) and becomes the stored photo's
// modification time. So a library photo's EXIF wins over everything the phone knows: it is the only
// one of the four written when the shutter fired, and the file a picker hands over is a fresh copy
// whose date may be the copy's.
//
// **Whole seconds, always**: EXIF's `SubSecTimeOriginal` is not read, and the format below has no
// fraction field, so the clock's and the file's are cut off. The export's zip keeps a photo's time to
// the second (the plan's P13 finding 9), so a fraction would be lost on the file at the lab, and a
// `--force` re-read there, which stamps the shot from the photo's time, would stamp another instant
// than the phone did. Two shots in one second cannot happen at the bay, where a shot takes longer.
//
// Foundation and ImageIO only, and no UIKit or Flutter (C6), so `app/tool/reader_check.sh` compiles
// this file for macOS and pins each route on JPEGs it writes.

import Foundation
import ImageIO

/// A namespace, as `ScreenReader` is, so nothing here meets a name in the Runner's other files.
enum PhotoTime {
    /// Which of C9's sources answered, so a check can say which route it pinned and a failure says
    /// which one moved.
    enum Route: String {
        case exifWithOffset = "EXIF date with its offset"
        case exifInDeviceOffset = "EXIF date in the device's offset"
        case fileDate = "the file's date"
        case clock = "now"
    }

    /// The camera's answer: `date` (now, by default) in the device's offset at that moment.
    static func now(at date: Date = Date(), in timeZone: TimeZone = .current) -> String {
        rfc3339(date, in: timeZone)
    }

    /// A library photo's time, by C9's order. `timeZone` is the device's and `now` the clock's, each
    /// an argument so that the check can fix them.
    static func capturedAt(
        url: URL, timeZone: TimeZone = .current, now: Date = Date()
    ) -> (text: String, route: Route) {
        if let (original, offset) = exifOriginal(url) {
            if let zone = offset.flatMap(zone(forOffset:)), let date = exifDate(original, in: zone) {
                return (rfc3339(date, in: zone), .exifWithOffset)
            }
            // The date is a wall clock with no offset. The device's offset at that date, and not at
            // today's, so a summer photo imported in winter keeps its summer offset.
            if let date = exifDate(original, in: timeZone) {
                return (rfc3339(date, in: timeZone), .exifInDeviceOffset)
            }
        }
        if let modified = try? url.resourceValues(forKeys: [.contentModificationDateKey])
            .contentModificationDate
        {
            return (rfc3339(modified, in: timeZone), .fileDate)
        }
        return (rfc3339(now, in: timeZone), .clock)
    }

    /// `2026-11-05T19:12:03-05:00`, and `+00:00` rather than `Z` for UTC, so that the text always says
    /// an offset was known. Rust's `parse_now` reads both, and refuses a time with none. A fraction of
    /// a second is cut off, never rounded up into the next second.
    static func rfc3339(_ date: Date, in timeZone: TimeZone) -> String {
        let formatter = posixFormatter("yyyy-MM-dd'T'HH:mm:ssxxxxx", timeZone)
        return formatter.string(from: date)
    }

    // MARK: - EXIF

    /// The primary image's `DateTimeOriginal`, and its `OffsetTimeOriginal` if one is written. The
    /// primary image, because that is the one `ScreenReader` reads from a HEIC holding several.
    static func exifOriginal(_ url: URL) -> (String, String?)? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil }
        let index = CGImageSourceGetPrimaryImageIndex(source)
        guard
            let properties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any],
            let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any],
            let original = exif[kCGImagePropertyExifDateTimeOriginal] as? String
        else { return nil }
        return (original, exif[kCGImagePropertyExifOffsetTimeOriginal] as? String)
    }

    /// EXIF's `2026:11:05 19:12:03` as a wall clock in `zone`, or `nil` for anything else. A camera
    /// that knows no date writes blanks in that shape, and those fall through to the file's date.
    /// Strict, so that `2026:02:30` is refused rather than rolled into March.
    static func exifDate(_ text: String, in zone: TimeZone) -> Date? {
        let formatter = posixFormatter("yyyy:MM:dd HH:mm:ss", zone)
        formatter.isLenient = false
        guard let date = formatter.date(from: text), formatter.string(from: date) == text else {
            return nil
        }
        return date
    }

    /// EXIF's `-05:00` as a fixed zone. Anything else, blanks included, is no offset: a fixed zone
    /// from a misread would put the photo at the wrong instant with nothing to show it.
    static func zone(forOffset text: String) -> TimeZone? {
        let characters = Array(text)
        let digit = { (character: Character) in character.isASCII && character.isNumber }
        guard characters.count == 6, characters[0] == "+" || characters[0] == "-",
            characters[3] == ":",
            characters[1...2].allSatisfy(digit), characters[4...5].allSatisfy(digit),
            let hours = Int(String(characters[1...2])), let minutes = Int(String(characters[4...5])),
            hours <= 18, minutes < 60
        else { return nil }
        let seconds = (hours * 3600 + minutes * 60) * (characters[0] == "-" ? -1 : 1)
        return TimeZone(secondsFromGMT: seconds)
    }

    // MARK: - Helpers

    /// A fixed-format formatter in the Gregorian calendar with the POSIX locale, so that a phone set to
    /// another calendar or to 12-hour time cannot change the spelling.
    static func posixFormatter(_ format: String, _ zone: TimeZone) -> DateFormatter {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = zone
        formatter.dateFormat = format
        return formatter
    }
}
