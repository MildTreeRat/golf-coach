// PhoneChannel.swift: the app's one native channel, `golf/phone`, and the only Swift that Dart
// calls. [M38 P18]
//
// One channel and this repo's Swift rather than a plugin (the plan's rule 10 and C4): the app's
// runtime dependencies are the Flutter SDK and the bridge, and every native thing it does is here, in
// a file a person can read whole. P18 landed `documentsDir` and `readScreen`, and P20 the four that
// `PhotoSource` answers: `cameraAvailable`, `takePhoto`, `pickPhotos` and `share`.
//
// What crosses is strings and integers. The boxes cross as the JSON text `ScreenReader` wrote, and
// Dart hands that text to Rust unparsed (rule 9), so no box number is ever a Dart double.

import Flutter
import Foundation

/// A namespace, as `ScreenReader` is, so that nothing here meets a name in the Runner's other files.
enum PhoneChannel {
    static let name = "golf/phone"

    /// Registered once, from `didInitializeImplicitFlutterEngine`, on the application registrar's
    /// messenger (C4, as the planning spike ran it). The messenger keeps the handler, so the channel
    /// object itself need not be kept.
    static func register(with messenger: FlutterBinaryMessenger) {
        let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
        channel.setMethodCallHandler { call, result in
            switch call.method {
            case "documentsDir":
                documentsDir(result)
            case "readScreen":
                readScreen(call.arguments, result)
            case "cameraAvailable":
                result(PhotoSource.cameraAvailable)
            case "takePhoto":
                PhotoSource.takePhoto { taken in
                    // A cancel is `null`, and not an error: the golfer chose it.
                    answer(result, taken.map { taken -> Any in
                        guard let taken else { return NSNull() }
                        return photo(taken)
                    })
                }
            case "pickPhotos":
                PhotoSource.pickPhotos { picked in
                    answer(
                        result,
                        picked.map { pick in
                            ["photos": pick.photos.map(photo), "skipped": pick.skipped] as [String: Any]
                        })
                }
            case "share":
                guard let arguments = call.arguments as? [String: Any],
                    let path = arguments["path"] as? String
                else {
                    result(FlutterError(code: "bad_arguments", message: "share takes {path: String}", details: nil))
                    return
                }
                PhotoSource.share(path: path) { answer(result, $0) }
            default:
                result(FlutterMethodNotImplemented)
            }
        }
    }

    /// `{"path": String, "capturedAt": String}`: the file under `tmp/`, and its time as RFC 3339 with an
    /// offset (C9), which Dart passes to `record_photo` as it stands.
    private static func photo(_ photo: PhotoSource.Photo) -> [String: Any] {
        ["path": photo.path, "capturedAt": photo.capturedAt]
    }

    /// A `PhotoSource` answer as the channel's: the value, or its refusal as a `FlutterError` with the
    /// code and sentence `PhotoSource` gave, which Dart receives as a `PlatformException`.
    private static func answer<T>(_ result: FlutterResult, _ outcome: Result<T, PhotoSource.Failure>) {
        switch outcome {
        case .success(let value):
            result(value)
        case .failure(let failure):
            result(FlutterError(code: failure.code, message: failure.message, details: nil))
        }
    }

    /// The app's Documents directory, as a path. The data root under it (`processed/`, C7) is Dart's
    /// to name and Rust's to make, so this says only where the container is, which on a reinstall
    /// moves.
    private static func documentsDir(_ result: @escaping FlutterResult) {
        guard
            let url = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        else {
            result(FlutterError(code: "no_documents", message: "iOS names no Documents directory", details: nil))
            return
        }
        result(url.path)
    }

    /// `{"path": String}` to `{"json": String, "msVision": Int}`, `ScreenReader.read`'s answer as it
    /// stands.
    ///
    /// **Off the main thread**, because the read is synchronous and the spike measured it at 1.26 s on
    /// the Simulator: on the main thread it would freeze the UI for that long (P17 finding 7). The
    /// reply goes back on the main thread, where Flutter's channels expect it.
    ///
    /// A photo that opens and holds no text is not an error: it answers an empty `boxes`, and Rust
    /// calls that `Failed`. Only a photo that cannot be opened or rendered is a `FlutterError`, code
    /// `unreadable`, with `ScreenReader`'s sentence, which Dart receives as a `PlatformException`.
    private static func readScreen(_ arguments: Any?, _ result: @escaping FlutterResult) {
        guard let arguments = arguments as? [String: Any], let path = arguments["path"] as? String
        else {
            result(FlutterError(code: "bad_arguments", message: "readScreen takes {path: String}", details: nil))
            return
        }
        DispatchQueue.global(qos: .userInitiated).async {
            let answer: Any
            do {
                let read = try ScreenReader.read(path: path)
                answer = ["json": read.json, "msVision": read.msVision] as [String: Any]
            } catch let failure as ScreenReader.Failure {
                answer = FlutterError(code: "unreadable", message: failure.description, details: nil)
            } catch {
                answer = FlutterError(code: "unreadable", message: "\(error)", details: nil)
            }
            DispatchQueue.main.async { result(answer) }
        }
    }
}
