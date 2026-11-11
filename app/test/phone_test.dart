import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/phone/phone.dart';

// `ChannelPhone` against a mocked `golf/phone`, with no Swift: what each answer Swift writes becomes in
// Dart, and `Photo.discard`. [M38 P20]
//
// The other half, that Swift writes those answers, is `integration_test/phone_test.dart` as far as a
// Simulator reaches, and P30 on the iPhone for what a finger has to drive.

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  /// Answers each method by [answers], and logs each call with its arguments.
  List<MethodCall> mock(Map<String, Object? Function(MethodCall)> answers) {
    final calls = <MethodCall>[];
    messenger.setMockMethodCallHandler(ChannelPhone.channel, (call) async {
      calls.add(call);
      final answer = answers[call.method];
      if (answer == null) {
        throw MissingPluginException(call.method);
      }
      return answer(call);
    });
    addTearDown(
      () => messenger.setMockMethodCallHandler(ChannelPhone.channel, null),
    );
    return calls;
  }

  const phone = ChannelPhone();

  test('a camera photo carries Swift\'s path and time as they stand, from the camera', () async {
    mock({
      'takePhoto': (_) => {
        'path': '/tmp/camera/camera-20261105-191203.jpg',
        'capturedAt': '2026-11-05T19:12:03-05:00',
      },
    });

    final photo = await phone.takePhoto();

    expect(photo, isNotNull);
    expect(photo!.path, '/tmp/camera/camera-20261105-191203.jpg');
    expect(photo.capturedAt, '2026-11-05T19:12:03-05:00');
    expect(photo.source, ReadSource.camera);
  });

  test('a cancelled camera is null, not an error', () async {
    mock({'takePhoto': (_) => null});

    expect(await phone.takePhoto(), isNull);
  });

  test('a refusal reaches Dart as Swift\'s code and sentence', () async {
    mock({
      'takePhoto': (_) => throw PlatformException(
        code: 'no_camera',
        message: 'This device has no camera to photograph the screen with.',
      ),
    });

    await expectLater(
      phone.takePhoto(),
      throwsA(
        isA<PlatformException>()
            .having((error) => error.code, 'code', 'no_camera')
            .having((error) => error.message, 'message', contains('no camera')),
      ),
    );
  });

  test('a pick keeps its order, comes from the library, and keeps what was skipped', () async {
    mock({
      'pickPhotos': (_) => {
        'photos': [
          {
            'path': '/tmp/library/a1-1/IMG_2889.heic',
            'capturedAt': '2026-10-07T18:01:02-04:00',
          },
          {
            'path': '/tmp/library/a1-2/IMG_2891.heic',
            'capturedAt': '2026-10-07T18:03:04-04:00',
          },
        ],
        'skipped': ['IMG_2890: iOS gave no file.'],
      },
    });

    final pick = await phone.pickPhotos();

    expect(pick.photos.map((photo) => photo.path), [
      '/tmp/library/a1-1/IMG_2889.heic',
      '/tmp/library/a1-2/IMG_2891.heic',
    ]);
    expect(pick.photos.map((photo) => photo.capturedAt), [
      '2026-10-07T18:01:02-04:00',
      '2026-10-07T18:03:04-04:00',
    ]);
    expect(
      pick.photos.map((photo) => photo.source),
      everyElement(ReadSource.library_),
    );
    expect(pick.skipped, ['IMG_2890: iOS gave no file.']);
  });

  test('a cancelled pick is empty', () async {
    mock({
      'pickPhotos': (_) => {'photos': <Object?>[], 'skipped': <Object?>[]},
    });

    final pick = await phone.pickPhotos();

    expect(pick.photos, isEmpty);
    expect(pick.skipped, isEmpty);
  });

  test(
    'share sends the path and answers whether an action was finished',
    () async {
      final calls = mock({'share': (call) => false});

      expect(await phone.share('/tmp/export.zip'), isFalse);
      expect(calls.single.arguments, {'path': '/tmp/export.zip'});
    },
  );

  test('cameraAvailable answers Swift\'s bool', () async {
    mock({'cameraAvailable': (_) => false});

    expect(await phone.cameraAvailable(), isFalse);
  });

  group('discard', () {
    late Directory scratch;
    setUp(() async {
      scratch = await Directory.systemTemp.createTemp('m38-discard-');
    });
    tearDown(() async {
      if (await scratch.exists()) {
        await scratch.delete(recursive: true);
      }
    });

    Photo photoAt(String path) => Photo(
      path: path,
      capturedAt: '2026-11-05T19:12:03-05:00',
      source: ReadSource.library_,
    );

    test('removes the copy and its directory once that is empty', () async {
      final directory = await Directory('${scratch.path}/a1-1').create();
      final file = await File('${directory.path}/IMG_2889.heic')
          .writeAsString('x');

      await photoAt(file.path).discard();

      expect(await file.exists(), isFalse);
      expect(await directory.exists(), isFalse);
    });

    test('leaves a directory that still holds another photo', () async {
      final file = await File('${scratch.path}/camera-1.jpg')
          .writeAsString('x');
      final other = await File('${scratch.path}/camera-2.jpg')
          .writeAsString('y');

      await photoAt(file.path).discard();

      expect(await file.exists(), isFalse);
      expect(await other.exists(), isTrue);
    });

    test('does not throw for a photo a failed read already moved', () async {
      final directory = await Directory('${scratch.path}/a1-2').create();

      await photoAt('${directory.path}/gone.heic').discard();

      expect(await directory.exists(), isFalse);
    });
  });
}
