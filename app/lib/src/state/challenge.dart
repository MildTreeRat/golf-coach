import 'package:flutter/foundation.dart';

import '../core/core_api.dart';
import 'app_model.dart';

/// The shapes a challenge draws over: the ticked ones in [TargetShape]'s own order, whatever order they
/// were ticked in, or none when no challenge is on. [M38 P23]
///
/// A fixed order is what lets a test name a draw by its index, and it costs a draw nothing: each tick
/// is one index of `nextInt(n)`, so the order changes which number names a shape and not how often it
/// comes up.
List<TargetShape> challengeTicks(AppState state) => state.challenge
    ? [
        for (final shape in TargetShape.values)
          if (state.challengeShapes.contains(shape)) shape,
      ]
    : const [];

/// D11's challenge: a call, drawn at random over the ticked shapes, shown large until a photo is
/// recorded under it. [M38 P23]
///
/// **The call lives here and not in `state.json`** (the plan's P6 finding 3). `record_photo` takes it as
/// `challenge_call` and stamps the shot `{shot_shaping, call}` over whatever mode the state holds, and
/// the state's mode is never touched. So turning the challenge off restores the prior mode by doing
/// nothing to it, and a relaunch under a challenge draws a fresh call rather than showing a stale one.
///
/// **Each call is uniform over the ticks** (D11): `nextInt(n)` over the `n` ticked shapes, from the
/// model's injected `Random` (C15), so a test can name each draw. A call changes on exactly three
/// things: a photo recorded under it ([recorded], `Read` or `Flagged`), the challenge turning off or on,
/// and its own shape being unticked, since a call that is no longer ticked cannot stand. Ticking another
/// shape keeps it, because it shows until the photo is recorded. A failed read, a duplicate, a refusal
/// and a photo Swift cannot open keep it too, since nothing was recorded under it (`RecordFlow` decides
/// which, since it alone knows the status).
///
/// It follows the model rather than being told about the change sheet's edits, so whatever writes the
/// state, the sheet or a relaunch reading it, the call agrees with it.
class Challenge extends ChangeNotifier {
  Challenge({required this.model}) {
    model.addListener(_follow);
    _follow();
  }

  final AppModel model;
  TargetShape? _call;

  /// The shape the next photo is to be hit as, or none when no challenge is on.
  TargetShape? get call => _call;

  /// A photo was recorded under [call], so the next one is drawn: from the ticks as they are now, and
  /// possibly the same shape again, since each draw is its own.
  void recorded() {
    final ticks = challengeTicks(model.state);
    _call = ticks.isEmpty ? null : _draw(ticks);
    notifyListeners();
  }

  void _follow() {
    final ticks = challengeTicks(model.state);
    final held = _call;
    final next = ticks.isEmpty
        ? null
        : held != null && ticks.contains(held)
        ? held
        : _draw(ticks);
    if (next != held) {
      _call = next;
      notifyListeners();
    }
  }

  TargetShape _draw(List<TargetShape> ticks) =>
      ticks[model.random.nextInt(ticks.length)];

  @override
  void dispose() {
    model.removeListener(_follow);
    super.dispose();
  }
}
