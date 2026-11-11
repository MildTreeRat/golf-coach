// PhotoSource.swift: where a photo comes from, the camera or the library, and the share sheet an
// export leaves by. [M38 P20]
//
// The plan's D5, C9 and C10. Each answer is a file under the app's `tmp/` and the time it was taken,
// and `PhoneChannel` carries both to Dart, which hands them to `record_photo` as they stand.
//
// **The camera is `UIImagePickerController` with its own controls hidden** (C10): an overlay holds a
// shutter, a cancel and the words "Fill the frame with the tiles", and the shutter calls
// `takePicture()`, so one tap goes straight to a read with no "Use Photo" step between swings. The
// image is stored as a JPEG at quality 0.95 with its Orientation tag kept, and not turned, so that the
// reader turns it once as it turns every photo (`ScreenReader.decode`), and a landscape photo of a wide
// monitor reads upright with the app's UI held portrait.
//
// **The library is `PHPickerViewController`**, any number of images, `.current` and
// `loadFileRepresentation`, so an original HEIC stays a HEIC and its bytes are what is stored and
// hashed. It needs no photo-library permission, since the picker runs outside the app and hands over
// only what was picked. A Live Photo arrives as its still (D4).
//
// **Neither can be driven from a test**, on the Simulator or anywhere, since the camera's prompt, a
// native picker and the share sheet each wait for a finger. The iOS 26.2 Simulator answers
// `cameraAvailable` true, and `takePhoto` there raises the system's camera prompt (the plan's P20
// findings), so `no_camera` is reached by no host this repo has. What a test reaches is `share`'s
// refusals before anything is presented, and the times, which `PhotoTime` answers and
// `app/tool/reader_check.sh` pins. The rest is unproven until P30, on the iPhone.
//
// Everything here runs on the main thread, where UIKit wants it, except the JPEG's encode and the
// library's copies, which run where noted and come back to the main thread to reply.

import AVFoundation
import Foundation
import ImageIO
import PhotosUI
import UIKit
import UniformTypeIdentifiers

/// A namespace, as `ScreenReader` is.
enum PhotoSource {
    /// A refusal, with the code Dart's `PlatformException` carries and a sentence a person can act on.
    struct Failure: Error {
        let code: String
        let message: String
    }

    /// A photo as a file the app owns, under `tmp/`, and when it was taken (C9).
    ///
    /// **The file name is the photo's own** where the photo has one, because the manifest records it as
    /// `original_filename` and the bay report finds a re-encoded copy by its stem (the plan's P15
    /// finding 10). So each library photo is copied into a directory of its own, and two photos
    /// sharing a name cannot meet.
    struct Photo {
        let path: String
        let capturedAt: String
    }

    /// What a library pick answered: the photos in the order picked, and a sentence for each one iOS
    /// would not hand over, so that a photo cannot leave a batch without the golfer being told.
    struct Pick {
        var photos: [Photo]
        var skipped: [String]
    }

    /// The presentation in progress, held because a picker's delegate is weak, and so that a second
    /// call while one is up is refused rather than presented over it. **Cleared when the presentation
    /// has gone, and not when it answers**: each answers as soon as it can, so a read starts while the
    /// camera or the picker slides away, and something presented during that slide would be presented
    /// from a controller on its way out, which UIKit ignores. Read and written on the main thread only.
    private static var active: NSObject?

    // MARK: - Camera

    static var cameraAvailable: Bool { UIImagePickerController.isSourceTypeAvailable(.camera) }

    /// The camera, behind C10's overlay. Answers `nil` if the golfer cancels.
    static func takePhoto(_ done: @escaping (Result<Photo?, Failure>) -> Void) {
        guard cameraAvailable else {
            done(
                .failure(
                    Failure(
                        code: "no_camera",
                        message: "This device has no camera to photograph the screen with. "
                            + "Import the photo from the library instead.")))
            return
        }
        guard active == nil else { return done(.failure(busy)) }
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            presentCamera(done)
        case .notDetermined:
            // The prompt answers on a queue of its own, and the picker has to be made on the main one.
            AVCaptureDevice.requestAccess(for: .video) { granted in
                DispatchQueue.main.async {
                    granted ? presentCamera(done) : done(.failure(cameraDenied))
                }
            }
        default:
            done(.failure(cameraDenied))
        }
    }

    private static func presentCamera(_ done: @escaping (Result<Photo?, Failure>) -> Void) {
        guard active == nil else { return done(.failure(busy)) }
        guard let top = topController() else { return done(.failure(noWindow)) }
        let session = CameraSession(reply: done, closed: { active = nil })
        active = session
        session.present(from: top)
    }

    // MARK: - Library

    /// The library picker, any number of images. Answers an empty pick if the golfer cancels.
    static func pickPhotos(_ done: @escaping (Result<Pick, Failure>) -> Void) {
        guard active == nil else { return done(.failure(busy)) }
        guard let top = topController() else { return done(.failure(noWindow)) }
        let session = LibrarySession(reply: { done(.success($0)) }, closed: { active = nil })
        active = session
        session.present(from: top)
    }

    // MARK: - Share

    /// The share sheet over the file at `path`. Answers whether the golfer finished an action with
    /// it, since a sheet closed with nothing chosen leaves the file unsent.
    static func share(path: String, _ done: @escaping (Result<Bool, Failure>) -> Void) {
        guard FileManager.default.fileExists(atPath: path) else {
            done(.failure(Failure(code: "no_file", message: "There is no file at \(path) to share.")))
            return
        }
        guard active == nil else { return done(.failure(busy)) }
        guard let top = topController() else { return done(.failure(noWindow)) }
        let sheet = UIActivityViewController(
            activityItems: [URL(fileURLWithPath: path)], applicationActivities: nil)
        // Answered once: the handler can be called again when an activity's own screen closes.
        var answered = false
        sheet.completionWithItemsHandler = { _, completed, _, error in
            guard !answered else { return }
            answered = true
            active = nil
            if let error {
                done(.failure(Failure(code: "share_failed", message: error.localizedDescription)))
            } else {
                done(.success(completed))
            }
        }
        // An iPad shows the sheet as a popover, which needs an anchor or UIKit stops the app.
        if let popover = sheet.popoverPresentationController {
            popover.sourceView = top.view
            popover.sourceRect = CGRect(x: top.view.bounds.midX, y: top.view.bounds.midY, width: 0, height: 0)
            popover.permittedArrowDirections = []
        }
        active = sheet
        top.present(sheet, animated: true)
    }

    // MARK: - Shared

    static let busy = Failure(
        code: "busy", message: "The camera, the library or the share sheet is already open.")
    static let noWindow = Failure(
        code: "no_window", message: "The app has no window to show the camera or the library in.")
    static let cameraDenied = Failure(
        code: "camera_denied",
        message: "Golf Coach may not use the camera. Allow it in Settings, Privacy & Security, "
            + "Camera, or import the photo from the library.")

    /// The controller to present from: the top of whatever the active window scene's root has
    /// presented, since UIKit ignores a presentation from a controller already presenting another.
    /// One on its way out is not the top, for the same reason. A presentation UIKit ignored would
    /// never answer and would hold `active` until the app is closed.
    static func topController() -> UIViewController? {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        let scene = scenes.first { $0.activationState == .foregroundActive } ?? scenes.first
        let window = scene?.windows.first { $0.isKeyWindow } ?? scene?.windows.first
        var top = window?.rootViewController
        while let presented = top?.presentedViewController, !presented.isBeingDismissed {
            top = presented
        }
        return top
    }

    /// `tmp/<kind>/`, made if missing. Under `tmp/` so that iOS may purge what a crash leaves, and
    /// outside the data root, so nothing here is ever taken for a stored photo or exported.
    static func scratchDirectory(_ kind: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            kind, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }
}

// MARK: - The camera's session

/// One camera presentation: the picker, its overlay, and the one answer it gives.
private final class CameraSession: NSObject, UIImagePickerControllerDelegate,
    UINavigationControllerDelegate
{
    private let picker = UIImagePickerController()
    private let reply: (Result<PhotoSource.Photo?, PhotoSource.Failure>) -> Void
    private let closed: () -> Void
    private var answered = false
    /// When the shutter was tapped: the camera's photo is "now" (C9), and now is the tap, not the
    /// moment the picker had finished processing the image.
    private var shutterAt: Date?
    private weak var shutter: UIButton?

    init(
        reply: @escaping (Result<PhotoSource.Photo?, PhotoSource.Failure>) -> Void,
        closed: @escaping () -> Void
    ) {
        self.reply = reply
        self.closed = closed
    }

    func present(from presenter: UIViewController) {
        picker.sourceType = .camera
        picker.cameraCaptureMode = .photo
        picker.cameraDevice = .rear
        // A flash on a screen is a white patch where the tiles were.
        picker.cameraFlashMode = .off
        picker.showsCameraControls = false
        picker.cameraOverlayView = overlay(frame: presenter.view.window?.bounds ?? presenter.view.bounds)
        picker.modalPresentationStyle = .fullScreen
        picker.delegate = self
        presenter.present(picker, animated: true)
    }

    /// C10's overlay: the words at the top, the shutter at the foot, and cancel beside it, each at
    /// least 56 points high, as the app's own touch targets are.
    private func overlay(frame: CGRect) -> UIView {
        let view = UIView(frame: frame)
        view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.backgroundColor = .clear

        let hint = PaddedLabel()
        hint.text = "Fill the frame with the tiles"
        hint.font = .preferredFont(forTextStyle: .headline)
        hint.textColor = .white
        hint.backgroundColor = UIColor.black.withAlphaComponent(0.55)
        hint.layer.cornerRadius = 10
        hint.layer.masksToBounds = true
        hint.textAlignment = .center

        let shutter = UIButton(type: .custom)
        shutter.backgroundColor = .white
        shutter.layer.cornerRadius = 38
        shutter.layer.borderColor = UIColor.lightGray.cgColor
        shutter.layer.borderWidth = 4
        shutter.accessibilityLabel = "Take the photo"
        shutter.addTarget(self, action: #selector(shoot), for: .touchUpInside)
        self.shutter = shutter

        let cancel = UIButton(type: .system)
        cancel.setTitle("Cancel", for: .normal)
        cancel.titleLabel?.font = .preferredFont(forTextStyle: .title3)
        cancel.tintColor = .white
        cancel.addTarget(self, action: #selector(cancelTapped), for: .touchUpInside)

        for subview in [hint, shutter, cancel] as [UIView] {
            subview.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview(subview)
        }
        let guide = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            hint.topAnchor.constraint(equalTo: guide.topAnchor, constant: 16),
            hint.centerXAnchor.constraint(equalTo: guide.centerXAnchor),
            hint.heightAnchor.constraint(greaterThanOrEqualToConstant: 44),
            shutter.bottomAnchor.constraint(equalTo: guide.bottomAnchor, constant: -24),
            shutter.centerXAnchor.constraint(equalTo: guide.centerXAnchor),
            shutter.widthAnchor.constraint(equalToConstant: 76),
            shutter.heightAnchor.constraint(equalToConstant: 76),
            cancel.leadingAnchor.constraint(equalTo: guide.leadingAnchor, constant: 24),
            cancel.centerYAnchor.constraint(equalTo: shutter.centerYAnchor),
            cancel.heightAnchor.constraint(greaterThanOrEqualToConstant: 56),
            cancel.widthAnchor.constraint(greaterThanOrEqualToConstant: 56),
        ])
        return view
    }

    @objc private func shoot() {
        // A second tap while the first is processing would be ignored by the picker with a warning, or
        // worse, answered twice.
        shutter?.isEnabled = false
        shutterAt = Date()
        picker.takePicture()
    }

    @objc private func cancelTapped() {
        finish(.success(nil))
    }

    func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
        finish(.success(nil))
    }

    func imagePickerController(
        _ picker: UIImagePickerController,
        didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]
    ) {
        guard let image = info[.originalImage] as? UIImage, let cgImage = image.cgImage else {
            finish(
                .failure(
                    PhotoSource.Failure(
                        code: "no_image", message: "The camera answered with no image. Take it again.")))
            return
        }
        let metadata = info[.mediaMetadata] as? [String: Any] ?? [:]
        let orientation = Self.tag(image.imageOrientation)
        let capturedAt = PhotoTime.now(at: shutterAt ?? Date())
        // The encode of a full-size image takes a visible moment, so off the main thread.
        DispatchQueue.global(qos: .userInitiated).async {
            let result: Result<PhotoSource.Photo?, PhotoSource.Failure>
            do {
                let url = try Self.store(
                    cgImage, metadata: metadata, orientation: orientation, capturedAt: capturedAt)
                result = .success(PhotoSource.Photo(path: url.path, capturedAt: capturedAt))
            } catch let failure as PhotoSource.Failure {
                result = .failure(failure)
            } catch {
                result = .failure(PhotoSource.Failure(code: "unwritable", message: "\(error)"))
            }
            DispatchQueue.main.async { self.finish(result) }
        }
    }

    /// Answers at once and closes the camera beside it, rather than after it, so that the read starts
    /// while the camera slides away and `ms_total` does not hold the animation.
    private func finish(_ result: Result<PhotoSource.Photo?, PhotoSource.Failure>) {
        guard !answered else { return }
        answered = true
        reply(result)
        picker.dismiss(animated: true, completion: closed)
    }

    /// The JPEG at quality 0.95 with the camera's metadata, its Orientation tag included and the
    /// pixels left as the sensor gave them (C10).
    private static func store(
        _ image: CGImage, metadata: [String: Any], orientation: CGImagePropertyOrientation,
        capturedAt: String
    ) throws -> URL {
        var properties = metadata
        properties[kCGImagePropertyOrientation as String] = orientation.rawValue
        // The TIFF dictionary carries a second Orientation, and two that disagree would leave which one
        // a reader honours to the reader.
        if var tiff = properties[kCGImagePropertyTIFFDictionary as String] as? [String: Any] {
            tiff[kCGImagePropertyTIFFOrientation as String] = orientation.rawValue
            properties[kCGImagePropertyTIFFDictionary as String] = tiff
        }
        properties[kCGImageDestinationLossyCompressionQuality as String] = 0.95

        let data = NSMutableData()
        guard
            let destination = CGImageDestinationCreateWithData(
                data as CFMutableData, UTType.jpeg.identifier as CFString, 1, nil)
        else { throw PhotoSource.Failure(code: "unwritable", message: "iOS cannot write a JPEG here.") }
        CGImageDestinationAddImage(destination, image, properties as CFDictionary)
        guard CGImageDestinationFinalize(destination) else {
            throw PhotoSource.Failure(code: "unwritable", message: "Writing the photo as a JPEG failed.")
        }

        let directory = try PhotoSource.scratchDirectory("camera")
        let url = try uniqueURL(in: directory, stem: "camera-" + fileStamp(capturedAt), suffix: "jpg")
        try (data as Data).write(to: url, options: .atomic)
        return url
    }

    /// `2026-11-05T19:12:03-05:00` as `20261105-191203`: the photo's own wall clock, so a name in the
    /// manifest says when it was taken without an offset to read.
    private static func fileStamp(_ capturedAt: String) -> String {
        let clock = capturedAt.prefix(19)
        return clock.filter { $0 != "-" && $0 != ":" }.replacingOccurrences(of: "T", with: "-")
    }

    private static func uniqueURL(in directory: URL, stem: String, suffix: String) throws -> URL {
        var url = directory.appendingPathComponent("\(stem).\(suffix)")
        var count = 2
        while FileManager.default.fileExists(atPath: url.path) {
            url = directory.appendingPathComponent("\(stem)-\(count).\(suffix)")
            count += 1
        }
        return url
    }

    /// UIKit's orientation as the EXIF tag it stands for.
    private static func tag(_ orientation: UIImage.Orientation) -> CGImagePropertyOrientation {
        switch orientation {
        case .up: return .up
        case .down: return .down
        case .left: return .left
        case .right: return .right
        case .upMirrored: return .upMirrored
        case .downMirrored: return .downMirrored
        case .leftMirrored: return .leftMirrored
        case .rightMirrored: return .rightMirrored
        @unknown default: return .up
        }
    }
}

/// A label with room around its words, so the hint reads as a plate over the preview.
private final class PaddedLabel: UILabel {
    private let inset = UIEdgeInsets(top: 8, left: 16, bottom: 8, right: 16)

    override func drawText(in rect: CGRect) {
        super.drawText(in: rect.inset(by: inset))
    }

    override var intrinsicContentSize: CGSize {
        let size = super.intrinsicContentSize
        return CGSize(
            width: size.width + inset.left + inset.right, height: size.height + inset.top + inset.bottom)
    }
}

// MARK: - The library's session

/// One library presentation: the picker, and the copies of what was picked.
private final class LibrarySession: NSObject, PHPickerViewControllerDelegate {
    private let reply: (PhotoSource.Pick) -> Void
    private let closed: () -> Void

    init(reply: @escaping (PhotoSource.Pick) -> Void, closed: @escaping () -> Void) {
        self.reply = reply
        self.closed = closed
    }

    func present(from presenter: UIViewController) {
        var configuration = PHPickerConfiguration()
        configuration.filter = .images
        configuration.selectionLimit = 0
        // The asset as it is, so an original HEIC stays a HEIC rather than becoming a JPEG on the way.
        configuration.preferredAssetRepresentationMode = .current
        let picker = PHPickerViewController(configuration: configuration)
        picker.delegate = self
        presenter.present(picker, animated: true)
    }

    func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
        picker.dismiss(animated: true, completion: closed)
        guard !results.isEmpty else { return reply(PhotoSource.Pick(photos: [], skipped: [])) }

        let batch: URL
        do {
            batch = try PhotoSource.scratchDirectory("library")
        } catch {
            return reply(
                PhotoSource.Pick(
                    photos: [], skipped: ["No photo could be copied: \(error.localizedDescription)"]))
        }
        let label = UUID().uuidString.prefix(8)
        let lock = NSLock()
        var slots = [Result<PhotoSource.Photo, PhotoSource.Failure>?](repeating: nil, count: results.count)
        let group = DispatchGroup()
        for (index, result) in results.enumerated() {
            let provider = result.itemProvider
            let name = provider.suggestedName ?? "photo \(index + 1)"
            // The still and not a Live Photo's bundle (D4), and a specific type so the file arrives as
            // the asset holds it.
            guard
                let type = provider.registeredTypeIdentifiers.first(where: { identifier in
                    guard let type = UTType(identifier) else { return false }
                    return type.conforms(to: .image) && !type.conforms(to: .livePhoto)
                })
            else {
                // Under the lock too: a handler from an earlier photo may be writing its own slot, and
                // two writes into one array race even at different indices.
                lock.lock()
                slots[index] = .failure(
                    PhotoSource.Failure(code: "skipped", message: "\(name): not an image iOS can hand over."))
                lock.unlock()
                continue
            }
            group.enter()
            // The URL is only valid inside this handler, so the copy and the time are taken here.
            _ = provider.loadFileRepresentation(forTypeIdentifier: type) { url, error in
                let slot: Result<PhotoSource.Photo, PhotoSource.Failure>
                if let url {
                    let directory = batch.appendingPathComponent(
                        "\(label)-\(index + 1)", isDirectory: true)
                    slot = Self.copy(url, as: name, into: directory, type: type)
                } else {
                    let why = error?.localizedDescription ?? "iOS gave no file"
                    slot = .failure(PhotoSource.Failure(code: "skipped", message: "\(name): \(why)."))
                }
                lock.lock()
                slots[index] = slot
                lock.unlock()
                group.leave()
            }
        }
        group.notify(queue: .main) {
            var pick = PhotoSource.Pick(photos: [], skipped: [])
            for slot in slots {
                switch slot {
                case .success(let photo): pick.photos.append(photo)
                case .failure(let failure): pick.skipped.append(failure.message)
                case nil: pick.skipped.append("A photo was neither copied nor refused.")
                }
            }
            self.reply(pick)
        }
    }

    /// The picked file under its own name in a directory of its own, and its time by C9 read from
    /// the picker's file before it is gone.
    private static func copy(
        _ url: URL, as suggested: String, into directory: URL, type: String
    ) -> Result<PhotoSource.Photo, PhotoSource.Failure> {
        let name = fileName(suggested, for: url, type: type)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let target = directory.appendingPathComponent(name)
            try FileManager.default.copyItem(at: url, to: target)
            let time = PhotoTime.capturedAt(url: url)
            return .success(PhotoSource.Photo(path: target.path, capturedAt: time.text))
        } catch {
            return .failure(
                PhotoSource.Failure(code: "skipped", message: "\(name): \(error.localizedDescription)."))
        }
    }

    /// The asset's own name with the file's suffix, so `IMG_2889` arrives as `IMG_2889.heic`.
    ///
    /// The picker's file is often named for its copy and not for the asset, which would leave the bay
    /// report nothing to find a re-encoded photo by (P15 finding 10), so the provider's
    /// `suggestedName` wins. It usually has no suffix, and a name that already ends in an image's
    /// suffix keeps it rather than gaining a second. A name's dots are not taken for a suffix
    /// otherwise: `Screenshot 10.03.22` is a name, not a `.22` file. A slash would make a path of it.
    static func fileName(_ suggested: String, for url: URL, type: String) -> String {
        let stem = suggested.replacingOccurrences(of: "/", with: "_")
        let given = (stem as NSString).pathExtension
        if !given.isEmpty, UTType(filenameExtension: given)?.conforms(to: .image) == true {
            return stem
        }
        let suffix =
            url.pathExtension.isEmpty
            ? (UTType(type)?.preferredFilenameExtension ?? "jpg") : url.pathExtension
        return "\(stem).\(suffix)"
    }
}
