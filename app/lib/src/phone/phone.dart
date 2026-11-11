import 'dart:io';

import 'package:flutter/services.dart';

import '../rust/third_party/app/api/record.dart' show ReadSource;

export '../rust/third_party/app/api/record.dart' show ReadSource;

/// What the phone does that Dart cannot, behind one interface so that a widget test can fake it with
/// no Swift (the plan's C22). [M38 P18, P20]
///
/// P18 landed the two calls a read needs, and P20 the camera, the library and the share sheet, on
/// the same channel. **The camera, the library and the share sheet cannot be driven by a test**, on
/// the Simulator or anywhere, since each waits for a finger: even the camera's permission prompt,
/// which the iOS 26.2 Simulator raises, since it reports a camera. So what each does once presented is
/// unproven until P30, on the iPhone.
abstract class Phone {
  /// The app's Documents directory. The data root is `processed/` under it (C7), and this names only
  /// the container, which a reinstall moves.
  Future<String> documentsDir();

  /// The photo at [path] through iOS Vision, M33's C0 reader as `ScreenReader.swift` ships it.
  ///
  /// A photo holding no text answers a read with no boxes, which `record_photo` calls `Failed`. A
  /// photo that cannot be opened at all throws a [PlatformException] with code `unreadable` and the
  /// reader's sentence.
  Future<ScreenRead> readScreen(String path);

  /// Whether this phone has a camera to photograph the screen with, as UIKit answers it. Without one
  /// the Record tab offers the library alone. **The iOS 26.2 Simulator answers `true`**, though the
  /// plan expected otherwise, so a Simulator test that must not meet the camera's prompt overrides
  /// this rather than relying on it.
  Future<bool> cameraAvailable();

  /// The camera behind C10's overlay: one tap of its shutter is the photo, with no "Use Photo" step.
  /// `null` if the golfer cancelled.
  ///
  /// Throws a [PlatformException] with Swift's sentence as its message: `no_camera`, `camera_denied`
  /// (the golfer said no to the camera, and Settings is where that changes), `busy` (the camera, the
  /// library or the share sheet is already up), `no_window`, `no_image` or `unwritable`.
  Future<Photo?> takePhoto();

  /// The library picker, any number of images, in the order picked. An empty pick if the golfer
  /// cancelled. A photo iOS would not hand over is a sentence in [PhotoPick.skipped] rather than an
  /// error, so one bad photo does not cost the rest of the batch.
  ///
  /// Throws a [PlatformException] coded `busy` or `no_window`, as [takePhoto] does.
  Future<PhotoPick> pickPhotos();

  /// The share sheet over the file at [path]. `true` if the golfer finished an action with it, and
  /// `false` if the sheet was closed with nothing chosen, so the file was not sent.
  ///
  /// Throws a [PlatformException] coded `no_file`, `busy`, `no_window` or `share_failed`.
  Future<bool> share(String path);
}

/// One photo to read: a copy the app owns under its `tmp/`, and when it was taken.
class Photo {
  const Photo({
    required this.path,
    required this.capturedAt,
    required this.source,
  });

  /// The copy, **under the photo's own file name**, since `record_photo` records that name as the
  /// manifest's `original_filename`, and the bay report finds a re-encoded copy by its stem (the
  /// plan's P15 finding 10). A camera photo is named for when it was taken.
  final String path;

  /// RFC 3339 with an offset and whole seconds, Swift's answer by C9, and passed to `record_photo` as
  /// `captured_at` as it stands. It is never the Dart clock's: a library photo's time is the one in
  /// its EXIF.
  final String capturedAt;

  /// Which button it came from, as `log_read` spells it. Dart knows this, since it chose the call,
  /// so Swift does not say it.
  final ReadSource source;

  /// Deletes the copy, and its directory if that is then empty, once `record_photo` has answered.
  ///
  /// Nothing else does: `record_photo` moves the photo only on `Failed` (to `.app/failed/`) and leaves
  /// it on every other answer, and a batch of HEICs would otherwise sit in `tmp/` until iOS purges it
  /// (the plan's P6 finding 12). Never throws, since a `Failed` read has already moved the file and
  /// the directory may hold another photo. **Call it only on a photo [Phone] answered**, never on a
  /// test's capture in the repo.
  Future<void> discard() async {
    final file = File(path);
    try {
      await file.delete();
    } on FileSystemException {
      // Moved by a failed read, or already gone.
    }
    try {
      // Not recursive, so a directory holding anything else stays, which is the check.
      await file.parent.delete();
    } on FileSystemException {
      // Not empty, or already gone.
    }
  }
}

/// What a library pick answered.
class PhotoPick {
  const PhotoPick({required this.photos, required this.skipped});

  static const none = PhotoPick(photos: [], skipped: []);

  /// The photos copied, in the order picked.
  final List<Photo> photos;

  /// A sentence for each photo picked and not copied, naming it and saying why, as Swift wrote it.
  final List<String> skipped;
}

/// One read as Swift answered it.
class ScreenRead {
  const ScreenRead({required this.json, required this.msVision});

  /// The boxes as `ScreenReader` wrote them. **Passed to Rust as it stands, and never parsed here**
  /// (rule 9), because a Dart double would be one more place a colour's bits could move.
  final String json;

  /// The whole Swift call, the decode and the colour scan included: the Swift side's share of a
  /// read's `ms_total`, beside Rust's `ms_rust` (P17 finding 7).
  final int msVision;
}

/// [Phone] over the app's own Swift, on the one channel `golf/phone` (C4). Not final, because the
/// integration tests extend it to hand the library picker known photos, which a native picker cannot
/// be driven to do (C22).
class ChannelPhone implements Phone {
  const ChannelPhone();

  static const channel = MethodChannel('golf/phone');

  @override
  Future<String> documentsDir() async {
    final path = await channel.invokeMethod<String>('documentsDir');
    return path!;
  }

  @override
  Future<ScreenRead> readScreen(String path) async {
    final answer = await channel.invokeMapMethod<String, Object?>(
      'readScreen',
      {'path': path},
    );
    return ScreenRead(
      json: answer!['json']! as String,
      msVision: answer['msVision']! as int,
    );
  }

  @override
  Future<bool> cameraAvailable() async {
    final available = await channel.invokeMethod<bool>('cameraAvailable');
    return available!;
  }

  @override
  Future<Photo?> takePhoto() async {
    final answer = await channel.invokeMapMethod<String, Object?>('takePhoto');
    return answer == null ? null : photoFrom(answer, ReadSource.camera);
  }

  @override
  Future<PhotoPick> pickPhotos() async {
    final answer = await channel.invokeMapMethod<String, Object?>('pickPhotos');
    if (answer == null) {
      return PhotoPick.none;
    }
    return PhotoPick(
      photos: [
        for (final photo in answer['photos']! as List<Object?>)
          photoFrom(
            (photo! as Map<Object?, Object?>).cast<String, Object?>(),
            ReadSource.library_,
          ),
      ],
      skipped: (answer['skipped']! as List<Object?>).cast<String>(),
    );
  }

  @override
  Future<bool> share(String path) async {
    final completed = await channel.invokeMethod<bool>('share', {'path': path});
    return completed!;
  }

  /// `{path, capturedAt}` as Swift's `PhoneChannel.photo` writes it.
  static Photo photoFrom(Map<String, Object?> answer, ReadSource source) =>
      Photo(
        path: answer['path']! as String,
        capturedAt: answer['capturedAt']! as String,
        source: source,
      );
}
