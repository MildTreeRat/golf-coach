import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  // The implicit engine is Flutter 3.47's default, so this, and not `didFinishLaunching`, is the first
  // moment a messenger exists. The registrant stays: it is how `integration_test`'s plugin, a dev
  // dependency and the only plugin here (rule 10), reaches the engine.
  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    PhoneChannel.register(with: engineBridge.applicationRegistrar.messenger())
  }
}
