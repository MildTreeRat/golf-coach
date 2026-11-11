import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import '../state/record_flow.dart';
import '../widgets/change_sheet.dart';
import '../widgets/shot_card.dart';
import 'review_screen.dart';

/// The one launch monitor this app reads (D3). A label, since nothing on the phone chooses it:
/// `app::DEVICE` is the constant behind it.
const deviceName = 'Full Swing';

/// The screen used between swings (D5): who, which club and as what, the last read's card, a large
/// shutter, and the library beneath it. [M38 P21]
///
/// It is laid out for a glance at arm's length: the header is what the next photo will be stamped
/// with, read from the stored state and not from a draft, so it cannot show one thing and stamp
/// another (C8). The card scrolls and the two ways in stay put, at the thumb.
///
/// **A flagged read is reviewed before the next photo** [M38 P22]: the flow pushes a forced review over
/// this tab, which only Save or Delete leaves (D9), and holds the shutter until it has. An unflagged
/// card opens the same review on a tap, and that one can be left.
///
/// **Under a challenge** [M38 P23] the call is shown large beneath the header, and the header names the
/// challenge in place of the mode, since every photo is stamped with the call and the stored mode is
/// only what turning it off goes back to (P21 finding 7).
class RecordScreen extends StatefulWidget {
  const RecordScreen({super.key, required this.model});

  final AppModel model;

  @override
  State<RecordScreen> createState() => _RecordScreenState();
}

class _RecordScreenState extends State<RecordScreen> {
  late final RecordFlow _flow = RecordFlow(model: widget.model);

  @override
  void initState() {
    super.initState();
    _flow
      ..review = _present
      ..start();
  }

  Future<ReviewEnd?> _present(
    ShotCard card, {
    required bool forced,
    ReviewPlace? place,
  }) async {
    if (!mounted) {
      return null;
    }
    return showReview(
      context,
      widget.model,
      card,
      forced: forced,
      place: place,
    );
  }

  @override
  void dispose() {
    _flow.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: Listenable.merge([widget.model, _flow, _flow.challenge]),
      builder: (context, _) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _Header(
            model: widget.model,
            busy: _flow.busy,
            call: _flow.challenge.call,
          ),
          const Divider(height: 1),
          if (_flow.challenge.call case final call?) _Call(call: call),
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: _Result(flow: _flow),
            ),
          ),
          _Capture(flow: _flow),
        ],
      ),
    );
  }
}

/// golfer · club · mode, the device, and the change sheet's button.
class _Header extends StatelessWidget {
  const _Header({required this.model, required this.busy, required this.call});

  final AppModel model;
  final bool busy;

  /// The challenge's call, or none.
  final TargetShape? call;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final state = model.state;
    final club = state.club;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text.rich(
                  key: const ValueKey('record-header'),
                  TextSpan(
                    children: [
                      TextSpan(text: model.golfer?.displayName ?? 'No golfer'),
                      const TextSpan(text: ' · '),
                      // A golfer's first tabs open with no club chosen (P19 finding 10), and a photo
                      // taken now would be refused, so the header says so in the colour of a problem.
                      club == null
                          ? TextSpan(
                              text: 'No club',
                              style: TextStyle(color: theme.colorScheme.error),
                            )
                          : TextSpan(text: club),
                      const TextSpan(text: ' · '),
                      // What the next photo is stamped as: the stored mode, or under a challenge the
                      // call, which the line beneath shows large.
                      TextSpan(
                        text: call == null
                            ? intentWord(state.mode, state.targetShape)
                            : 'Challenge',
                      ),
                    ],
                  ),
                  style: theme.textTheme.titleMedium,
                ),
                Text(deviceName, style: theme.textTheme.bodySmall),
              ],
            ),
          ),
          TextButton(
            key: const ValueKey('change'),
            // Not during a read, so a batch runs under the club and intent it started with (C14).
            onPressed: busy ? null : () => showChangeSheet(context, model),
            child: const Text('Change'),
          ),
        ],
      ),
    );
  }
}

/// The challenge's call, large enough to read from the hitting position, until a photo is recorded
/// under it (D11). [M38 P23]
class _Call extends StatelessWidget {
  const _Call({required this.call});

  final TargetShape call;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final on = theme.colorScheme.onPrimaryContainer;
    return Container(
      key: const ValueKey('challenge-call'),
      color: theme.colorScheme.primaryContainer,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Column(
        children: [
          Text(
            'Challenge: hit a',
            style: theme.textTheme.titleSmall?.copyWith(color: on),
          ),
          Text(
            shapeWord(call),
            key: const ValueKey('challenge-shape'),
            style: theme.textTheme.displayMedium?.copyWith(
              color: on,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}

/// The batch's progress, what the last tap ended on, and the last card.
class _Result extends StatelessWidget {
  const _Result({required this.flow});

  final RecordFlow flow;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final batch = flow.batch;
    final problem = flow.problem;
    final last = flow.last;
    final card = last?.card;
    final message = last?.message;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (batch != null) ...[
          Text(
            'Reading ${batch.index} of ${batch.total}',
            key: const ValueKey('batch'),
          ),
          const SizedBox(height: 8),
          LinearProgressIndicator(value: (batch.index - 1) / batch.total),
          const SizedBox(height: 16),
        ] else if (flow.busy) ...[
          const LinearProgressIndicator(),
          const SizedBox(height: 16),
        ],
        if (problem != null)
          _Notice(
            key: const ValueKey('problem'),
            text: problem,
            color: theme.colorScheme.errorContainer,
            onColor: theme.colorScheme.onErrorContainer,
          ),
        for (final note in flow.notes)
          _Notice(
            text: note,
            color: theme.colorScheme.surfaceContainerHighest,
            onColor: theme.colorScheme.onSurface,
          ),
        if (last == null)
          problem == null && flow.notes.isEmpty && !flow.busy
              ? Padding(
                  padding: const EdgeInsets.only(top: 24),
                  child: Text(
                    'Photograph the screen after each shot. The read shows here.',
                    textAlign: TextAlign.center,
                    style: theme.textTheme.bodyLarge,
                  ),
                )
              : const SizedBox.shrink()
        else if (card == null)
          // `Failed`, the one answer with no card: Rust's sentence says to retake.
          _Notice(
            key: const ValueKey('failed'),
            text: message ?? '',
            color: theme.colorScheme.errorContainer,
            onColor: theme.colorScheme.onErrorContainer,
          )
        else
          ShotCardView(
            card: card,
            message: message,
            onTap: flow.busy ? null : () => flow.open(card),
          ),
      ],
    );
  }
}

class _Notice extends StatelessWidget {
  const _Notice({
    super.key,
    required this.text,
    required this.color,
    required this.onColor,
  });

  final String text;
  final Color color;
  final Color onColor;

  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.only(bottom: 12),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: color,
      borderRadius: BorderRadius.circular(12),
    ),
    child: Text(text, style: TextStyle(color: onColor)),
  );
}

/// The shutter, large, and the library beneath it. With no camera the shutter is not there at all,
/// rather than there and refusing (D5).
class _Capture extends StatelessWidget {
  const _Capture({required this.flow});

  final RecordFlow flow;

  /// The shutter's side: well past the 56 every tap target gets, since it is the one tap between
  /// swings.
  static const shutterSize = 88.0;

  @override
  Widget build(BuildContext context) {
    final idle = !flow.busy;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          if (flow.camera == true) ...[
            SizedBox.square(
              dimension: shutterSize,
              child: FilledButton(
                key: const ValueKey('shutter'),
                onPressed: idle ? flow.takePhoto : null,
                style: FilledButton.styleFrom(
                  shape: const CircleBorder(),
                  padding: EdgeInsets.zero,
                  minimumSize: const Size.square(shutterSize),
                ),
                child: const Icon(Icons.photo_camera, size: 40),
              ),
            ),
            const SizedBox(height: 8),
          ],
          OutlinedButton.icon(
            key: const ValueKey('library'),
            onPressed: idle ? flow.importPhotos : null,
            icon: const Icon(Icons.photo_library_outlined),
            label: const Text('From library'),
          ),
        ],
      ),
    );
  }
}
