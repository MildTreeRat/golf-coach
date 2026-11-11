import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';

/// A practice mode as the app names it.
String modeWord(PracticeMode mode) => switch (mode) {
  PracticeMode.fundamentals => 'Fundamentals',
  PracticeMode.shotShaping => 'Shot shaping',
  PracticeMode.performance => 'Performance',
  PracticeMode.drill => 'Drill',
};

/// A target shape as the app names it.
String shapeWord(TargetShape shape) => switch (shape) {
  TargetShape.straight => 'Straight',
  TargetShape.draw => 'Draw',
  TargetShape.fade => 'Fade',
};

/// The mode a shot is stamped with, and its shape where it has one: `Shot shaping: Draw`.
String intentWord(PracticeMode mode, TargetShape? shape) =>
    shape == null ? modeWord(mode) : '${modeWord(mode)}: ${shapeWord(shape)}';

/// Whether the sheet may close on [state]: not on `shot_shaping` with no shape [M38 P21], and not on a
/// challenge with no shape ticked [M38 P23].
///
/// `set_state` stores that pair, since the sheet has to pass through it on the way to a shape (P5),
/// and `record_photo` refuses it in words that name a command-line flag (P6 finding 13). So the sheet
/// holds the golfer here until a shape is picked, rather than let the next photo be the one to say so.
/// The pair is asked under a challenge too, since it is what turning the challenge off goes back to. A
/// challenge with nothing ticked would draw no call, so the next photo would quietly take the stored
/// mode. The sheet never makes one, since the last tick cannot be taken off, and this keeps a state
/// read from elsewhere from closing on one.
bool sheetCanClose(AppState state) =>
    (state.mode != PracticeMode.shotShaping || state.targetShape != null) &&
    (!state.challenge || state.challengeShapes.isNotEmpty);

/// Whether the sheet offers the challenge: not under Drill, which a challenge excludes (C15), unless
/// one is already on, so that a state holding both can still be turned off. [M38 P23]
bool challengeOffered(AppState state) =>
    state.mode != PracticeMode.drill || state.challenge;

/// The Record tab's change sheet. Barrier taps ask [PopScope], and a drag would pop past it, so the
/// sheet is not draggable.
Future<void> showChangeSheet(BuildContext context, AppModel model) =>
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      enableDrag: false,
      builder: (_) => ChangeSheet(model: model),
    );

/// Who is hitting, with which club, and as what (D5, C15). [M38 P21]
///
/// **Each pick is saved to `state.json` as it is made**, through [AppModel.editState], since that is
/// what `record_photo` stamps (C8): there is no draft to lose and no Save to forget between swings.
/// The club picker lists the current golfer's bag alone (D7), and the shape is offered only under
/// `Shot shaping`, the one mode that takes one.
///
/// **The challenge** (D11, C15) [M38 P23] is a switch with a tick per shape. It is hidden under Drill,
/// and while it is on Drill cannot be picked, so the two never stand together. Turning it on with no
/// shapes stored ticks all three, and turning it off keeps the ticks for next time. At least one stays
/// ticked: the last tick cannot be taken off. The mode and shape above stay pickable under it, since
/// they are what turning it off goes back to, and nothing about the challenge is written into them.
///
/// One save at a time: each edit is worked from the state the model holds, so a second tap before the
/// first's answer would be worked from a state about to be replaced, and could undo it.
class ChangeSheet extends StatefulWidget {
  const ChangeSheet({super.key, required this.model});

  final AppModel model;

  @override
  State<ChangeSheet> createState() => _ChangeSheetState();
}

class _ChangeSheetState extends State<ChangeSheet> {
  bool _saving = false;
  String? _problem;

  Future<void> _save(Future<void> Function() change) async {
    setState(() {
      _saving = true;
      _problem = null;
    });
    try {
      await change();
    } on Refusal catch (refusal) {
      widget.model.logProblem('change sheet', refusal.message);
      if (mounted) {
        setState(() => _problem = refusal.message);
      }
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  void _edit(AppState Function(AppState state) edit) =>
      _save(() => widget.model.editState(edit));

  /// On ticks all three if none is stored (D11's default), and otherwise the ticks last used. Off leaves
  /// the ticks, and the mode was never moved, so it is what the next photo takes.
  void _pickChallenge(bool on) => _edit(
    (state) => state.copyWith(
      challenge: on,
      challengeShapes: on && state.challengeShapes.isEmpty
          ? TargetShape.values
          : null,
    ),
  );

  /// One shape ticked or unticked, stored in [TargetShape]'s order. The last tick's button is disabled,
  /// which is what keeps one on.
  void _tick(TargetShape shape) => _edit((state) {
    final ticked = state.challengeShapes.toSet();
    if (!ticked.remove(shape)) {
      ticked.add(shape);
    }
    return state.copyWith(
      challengeShapes: [
        for (final each in TargetShape.values)
          if (ticked.contains(each)) each,
      ],
    );
  });

  void _pickMode(PracticeMode mode) => _edit(
    (state) => state.copyWith(
      mode: mode,
      // A shape belongs to shot shaping alone, so leaving it leaves the shape, which would otherwise
      // sit hidden in the state and make the next photo a refusal.
      targetShape: mode == PracticeMode.shotShaping ? null : () => null,
    ),
  );

  @override
  Widget build(BuildContext context) {
    final model = widget.model;
    return ListenableBuilder(
      listenable: model,
      builder: (context, _) {
        final state = model.state;
        final canClose = sheetCanClose(state) && !_saving;
        final theme = Theme.of(context);
        return PopScope(
          canPop: canClose,
          child: SafeArea(
            child: SingleChildScrollView(
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _Heading('Golfer'),
                  _Choices(
                    children: [
                      for (final golfer in model.golfers)
                        _Choice(
                          key: ValueKey('pick-golfer-${golfer.playerId}'),
                          label: golfer.displayName,
                          selected: golfer.playerId == state.playerId,
                          onPressed: _saving
                              ? null
                              : () => _save(
                                  () => model.chooseGolfer(golfer.playerId),
                                ),
                        ),
                    ],
                  ),
                  _Heading('Club'),
                  if (model.bag.isEmpty)
                    const Text('This bag holds no club yet.')
                  else
                    _Choices(
                      children: [
                        for (final slot in model.bag)
                          _Choice(
                            key: ValueKey('pick-club-${slot.club}'),
                            label: slot.club,
                            selected: slot.club == state.club,
                            onPressed: _saving
                                ? null
                                : () => _edit(
                                    (state) =>
                                        state.copyWith(club: () => slot.club),
                                  ),
                          ),
                      ],
                    ),
                  _Heading('Mode'),
                  _Choices(
                    children: [
                      for (final mode in PracticeMode.values)
                        _Choice(
                          key: ValueKey('pick-mode-${mode.name}'),
                          label: modeWord(mode),
                          selected: mode == state.mode,
                          // A challenge's shots are tracked, and a drill's never are (C15).
                          onPressed:
                              _saving ||
                                  (state.challenge &&
                                      mode == PracticeMode.drill)
                              ? null
                              : () => _pickMode(mode),
                        ),
                    ],
                  ),
                  if (state.mode == PracticeMode.shotShaping) ...[
                    _Heading('Shape'),
                    _Choices(
                      children: [
                        for (final shape in TargetShape.values)
                          _Choice(
                            key: ValueKey('pick-shape-${shape.name}'),
                            label: shapeWord(shape),
                            selected: shape == state.targetShape,
                            onPressed: _saving
                                ? null
                                : () => _edit(
                                    (state) => state.copyWith(
                                      targetShape: () => shape,
                                    ),
                                  ),
                          ),
                      ],
                    ),
                    if (state.targetShape == null)
                      Padding(
                        padding: const EdgeInsets.only(top: 8),
                        child: Text(
                          'Pick the shape you are trying to hit.',
                          style: TextStyle(color: theme.colorScheme.error),
                        ),
                      ),
                  ],
                  if (challengeOffered(state)) ...[
                    _Heading('Challenge'),
                    SwitchListTile(
                      key: const ValueKey('challenge'),
                      contentPadding: EdgeInsets.zero,
                      title: const Text('Call each shot at random'),
                      subtitle: Text(
                        state.challenge
                            ? 'Each shot is hit to the call on Record. Off goes '
                                  'back to ${intentWord(state.mode, state.targetShape)}, '
                                  'and Drill waits until then.'
                            : 'From the shapes you tick, one call a shot.',
                      ),
                      value: state.challenge,
                      onChanged: _saving ? null : _pickChallenge,
                    ),
                    if (state.challenge)
                      _Choices(
                        children: [
                          for (final shape in TargetShape.values)
                            _Choice(
                              key: ValueKey('tick-shape-${shape.name}'),
                              label: shapeWord(shape),
                              selected: state.challengeShapes.contains(shape),
                              onPressed:
                                  _saving ||
                                      (state.challengeShapes.contains(shape) &&
                                          state.challengeShapes.length == 1)
                                  ? null
                                  : () => _tick(shape),
                            ),
                        ],
                      ),
                  ],
                  if (_problem != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 12),
                      child: Text(
                        _problem!,
                        style: TextStyle(color: theme.colorScheme.error),
                      ),
                    ),
                  const SizedBox(height: 20),
                  FilledButton(
                    onPressed: canClose
                        ? () => Navigator.of(context).pop()
                        : null,
                    child: const Text('Done'),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class _Heading extends StatelessWidget {
  const _Heading(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: 12, bottom: 8),
    child: Text(text, style: Theme.of(context).textTheme.titleSmall),
  );
}

class _Choices extends StatelessWidget {
  const _Choices({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) =>
      Wrap(spacing: 8, runSpacing: 8, children: children);
}

/// One choice. Buttons rather than chips, because the theme holds a button to the 56 a gloved tap
/// needs and a chip to nothing.
class _Choice extends StatelessWidget {
  const _Choice({
    super.key,
    required this.label,
    required this.selected,
    required this.onPressed,
  });

  final String label;
  final bool selected;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) => selected
      ? FilledButton.icon(
          onPressed: onPressed,
          icon: const Icon(Icons.check),
          label: Text(label),
        )
      : OutlinedButton(onPressed: onPressed, child: Text(label));
}
