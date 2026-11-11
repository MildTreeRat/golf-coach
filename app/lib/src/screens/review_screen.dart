import 'dart:io';

import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import '../widgets/change_sheet.dart' show intentWord;
import '../widgets/shot_card.dart' show CardRowView;

/// How a review ended, which whoever pushed it reads to replace or drop the card it holds. A back-out,
/// which only a review that is not forced allows, ends as `null`. [M38 P22]
sealed class ReviewEnd {
  const ReviewEnd();
}

/// Saved. [card] is the one `correct_shot` answered, and it replaces the card held, since it is the shot
/// as it now stands: unflagged, re-analysed, and each changed row `Corrected` (the plan's P7 finding 12).
final class ReviewSaved extends ReviewEnd {
  const ReviewSaved(this.card);

  final ShotCard card;
}

/// The entry is gone: deleted from here (D10), or found not to be on this phone when the review looked.
final class ReviewDeleted extends ReviewEnd {
  const ReviewDeleted();
}

/// Where a review stands in a batch's queue (C14): [index] of [total], counted from 1.
typedef ReviewPlace = ({int index, int total});

/// Every intent a shot can be stamped with, in the change sheet's order, as `set_shot_intent` takes it.
///
/// **Pairs, so that no choice is one Rust would refuse**: a shape goes with shot shaping alone, and shot
/// shaping needs one (C15, `intent_action`'s rule). The change sheet picks a mode and then a shape,
/// because the stored state may sit between the two while the sheet is up. A review has nothing to sit
/// between, since nothing is written until Save, so a half-chosen intent would only be a Save to refuse.
final List<(PracticeMode, TargetShape?)> shotIntents = [
  for (final mode in PracticeMode.values)
    if (mode == PracticeMode.shotShaping)
      for (final shape in TargetShape.values) (mode, shape)
    else
      (mode, null),
];

/// Push the review of [card] over whatever is showing, and wait for how it ended.
///
/// [forced] is D9's: a flagged read, which nothing leaves but Save or Delete. [place] is set when it is
/// one of a batch's queue.
///
/// Logged as `review_opened` (D16) [M38 P30b], here because both ways in, the Record tab's and a shot's
/// detail, come through this. A back-out, which only a review that is not forced allows, logs nothing,
/// so a `review_opened` with no `review_saved` or `shot_deleted` for its shot before the next one is a
/// review left as it was.
Future<ReviewEnd?> showReview(
  BuildContext context,
  AppModel model,
  ShotCard card, {
  required bool forced,
  ReviewPlace? place,
}) {
  model.logEvent(
    EventKind.reviewOpened,
    [
      shotOf(card.sessionId, card.swingId),
      if (forced) 'forced',
      if (place != null && place.total > 1) '${place.index} of ${place.total}',
    ].join(', '),
  );
  return Navigator.of(context).push<ReviewEnd>(
    MaterialPageRoute(
      fullscreenDialog: true,
      builder: (_) =>
          ReviewScreen(model: model, card: card, forced: forced, place: place),
    ),
  );
}

/// One shot as an event's detail names it, `<session>/<swing>`, the path its entry is stored at.
/// [M38 P30b]
String shotOf(String sessionId, String swingId) => '$sessionId/$swingId';

/// Ask before a delete, which nothing undoes: the photo in the entry goes with its read (D10, the plan's
/// P10 finding 10). True is a person's `Delete`, and anything else, the barrier included, is a keep.
///
/// One dialog for the review and the shot's detail screen (P25), so the two deletes say one thing.
/// Keyed `delete-keep` and `delete-confirm`. [M38 P22]
Future<bool> confirmDelete(BuildContext context, String swingId) async =>
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Delete shot $swingId?'),
        content: const Text(
          'Its photo and its read go from this phone, and nothing brings them back.',
        ),
        actions: [
          TextButton(
            key: const ValueKey('delete-keep'),
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Keep'),
          ),
          TextButton(
            key: const ValueKey('delete-confirm'),
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Delete'),
          ),
        ],
      ),
    ) ==
    true;

/// One shot against its photo, each value editable (D9, and C11 from a person's side). [M38 P22]
///
/// **It edits a draft and writes nothing until Save**, which calls `correct_shot` with a [FieldEdit] for
/// each row the person changed and for no other: `CardRow::printed` is six significant digits, so a row
/// sent back untouched could move a longer stored number (P7 finding 12). What a person types is what
/// the screen prints, in the label's unit and with the lit letter beside it, and Rust turns it into the
/// stored number (P7 finding 3), so nothing here converts, signs or checks a value (rule 8): a typo goes
/// to Rust as typed and comes back as its sentence, shown here, with nothing written.
///
/// **An empty field is a clear.** The clear control empties the field, so the two cannot be told apart
/// on screen, and they mean one thing. Sending an emptied field as typed would be `Some("")`, which Rust
/// refuses as not a number, and a refusal for what a person can see they did on purpose is no help.
///
/// A forced review has no close button and refuses the system's back, through [PopScope]. Save releases
/// it whether or not anything changed, since a person has now looked, and Delete removes it. If the entry
/// turns out not to be on this phone at all, there is nothing to review or release, and it lets go.
class ReviewScreen extends StatefulWidget {
  const ReviewScreen({
    super.key,
    required this.model,
    required this.card,
    required this.forced,
    this.place,
  });

  final AppModel model;

  /// The card as the person last saw it, whose rows are the draft's starting point and what a change is
  /// measured against.
  final ShotCard card;
  final bool forced;
  final ReviewPlace? place;

  @override
  State<ReviewScreen> createState() => _ReviewScreenState();
}

class _ReviewScreenState extends State<ReviewScreen> {
  late final Map<String, TextEditingController> _fields = {
    for (final row in widget.card.rows)
      row.field: TextEditingController(text: row.printed ?? ''),
  };
  late final Map<String, String?> _letters = {
    for (final row in widget.card.rows) row.field: row.letter,
  };

  ShotDetail? _detail;
  (PracticeMode, TargetShape?)? _storedIntent;
  (PracticeMode, TargetShape?)? _pickedIntent;
  bool _saving = false;
  String? _problem;

  /// `shot_detail`'s refusal, once it has said the entry is not on this phone.
  String? _goneBecause;
  bool get _gone => _goneBecause != null;

  AppModel get _model => widget.model;
  ShotCard get _card => widget.card;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final field in _fields.values) {
      field.dispose();
    }
    super.dispose();
  }

  /// The detail, for the photo and the intent the shot holds. A refusal here is an entry not on this
  /// phone (`shot_detail`'s one `Err`), which is how a review learns there is nothing left to review.
  Future<void> _load() async {
    try {
      final detail = await _model.core.shotDetail(
        root: _model.root,
        sessionId: _card.sessionId,
        swingId: _card.swingId,
      );
      if (mounted) {
        setState(() => _held(detail));
      }
    } on Refusal catch (refusal) {
      _model.logProblem('review', refusal.message);
      if (mounted) {
        setState(() => _goneBecause = refusal.message);
      }
    }
  }

  /// This review's shot, as an event names it.
  String get _shot => shotOf(_card.sessionId, _card.swingId);

  void _held(ShotDetail detail) {
    _detail = detail;
    _storedIntent = (detail.row.mode, detail.row.targetShape);
  }

  /// What Save sends for [row], or nothing if the person left it as it was.
  FieldEdit? _editOf(CardRow row) {
    final typed = _fields[row.field]!.text;
    if (typed.trim().isEmpty) {
      return row.printed == null ? null : FieldEdit(field: row.field);
    }
    final letter = _letters[row.field];
    if (typed == row.printed && letter == row.letter) {
      return null;
    }
    return FieldEdit(field: row.field, printed: typed, letter: letter);
  }

  /// The intent first, then the values, because `correct_shot` is what releases a flagged shot, so it is
  /// the last word: a refusal of either leaves the review up and the flag on. An intent stored before
  /// the values were refused stays stored, and is not sent again.
  Future<void> _save() async {
    setState(() {
      _saving = true;
      _problem = null;
    });
    try {
      final picked = _pickedIntent;
      if (picked != null && picked != _storedIntent) {
        final detail = await _model.core.setShotIntent(
          root: _model.root,
          sessionId: _card.sessionId,
          swingId: _card.swingId,
          mode: picked.$1,
          targetShape: picked.$2,
          now: _model.now(),
        );
        _model.logEvent(
          EventKind.intentSet,
          '$_shot ${[picked.$1.name, ?picked.$2?.name].join(' ')}',
        );
        _held(detail);
      }
      final edits = [for (final row in _card.rows) ?_editOf(row)];
      final card = await _model.core.correctShot(
        root: _model.root,
        sessionId: _card.sessionId,
        swingId: _card.swingId,
        edits: edits,
        now: _model.now(),
      );
      _model.logEvent(
        EventKind.reviewSaved,
        '$_shot, ${counted(edits.length, 'edit')}',
      );
      if (mounted) {
        Navigator.of(context).pop(ReviewSaved(card));
      }
    } on Refusal catch (refusal) {
      _model.logProblem('review', refusal.message);
      if (mounted) {
        setState(() => _problem = refusal.message);
      }
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  Future<void> _delete() async {
    final sure = await confirmDelete(context, _card.swingId);
    if (!sure || !mounted) {
      return;
    }
    setState(() {
      _saving = true;
      _problem = null;
    });
    try {
      await _model.core.deleteShot(
        root: _model.root,
        sessionId: _card.sessionId,
        swingId: _card.swingId,
      );
      _model.logEvent(EventKind.shotDeleted, _shot);
      if (mounted) {
        Navigator.of(context).pop(const ReviewDeleted());
      }
    } on Refusal catch (refusal) {
      _model.logProblem('delete', refusal.message);
      if (!mounted) {
        return;
      }
      setState(() => _problem = refusal.message);
      // `delete_shot` can refuse after the entry has gone, when the shot read from it would not go, and
      // then nothing is left to delete or save. Looking again is how a forced review learns that, rather
      // than hold the golfer on a screen every button of which now refuses.
      await _load();
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final place = widget.place;
    final problem = _problem;
    final idle = !_saving;
    return PopScope(
      canPop: idle && (!widget.forced || _gone),
      child: Scaffold(
        appBar: AppBar(
          automaticallyImplyLeading: !widget.forced,
          title: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Shot ${_card.swingId}'),
              if (place != null && place.total > 1)
                Text(
                  'Review ${place.index} of ${place.total}',
                  key: const ValueKey('review-place'),
                  style: theme.textTheme.bodySmall,
                ),
            ],
          ),
        ),
        body: SingleChildScrollView(
          padding: const EdgeInsets.only(bottom: 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              ShotPhoto(
                path: _detail?.row.photoPath,
                missing: _gone
                    ? 'This shot is not on this phone.'
                    : _detail == null
                    ? 'Finding the photo.'
                    : 'No photo is stored for this shot.',
              ),
              if (_card.flagged)
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                  child: Row(
                    children: [
                      Icon(Icons.flag, color: colors.error),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          'Check each value against the photo, then save.',
                        ),
                      ),
                    ],
                  ),
                ),
              for (final warning in _card.warnings)
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
                  child: Text(
                    warning,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: colors.error,
                    ),
                  ),
                ),
              const SizedBox(height: 8),
              for (final row in _card.rows)
                _EditRow(
                  row: row,
                  field: _fields[row.field]!,
                  letter: _letters[row.field],
                  enabled: idle && !_gone,
                  onLetter: (letter) =>
                      setState(() => _letters[row.field] = letter),
                  onClear: () => setState(() {
                    _fields[row.field]!.clear();
                    _letters[row.field] = null;
                  }),
                ),
              IntentPicker(
                shown: _pickedIntent ?? _storedIntent,
                enabled: idle && !_gone && _storedIntent != null,
                onPick: (intent) => setState(() => _pickedIntent = intent),
              ),
            ],
          ),
        ),
        bottomNavigationBar: SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Save's or Delete's refusal, in Rust's words, beside the buttons that met it.
                if (problem != null)
                  ProblemNote(
                    key: const ValueKey('review-problem'),
                    text: problem,
                  ),
                if (_goneBecause case final gone?)
                  ProblemNote(key: const ValueKey('review-gone'), text: gone),
                if (_gone)
                  FilledButton(
                    key: const ValueKey('review-close'),
                    onPressed: () =>
                        Navigator.of(context).pop(const ReviewDeleted()),
                    child: const Text('Close'),
                  )
                else
                  Row(
                    children: [
                      Expanded(
                        child: OutlinedButton.icon(
                          key: const ValueKey('review-delete'),
                          onPressed: idle ? _delete : null,
                          style: OutlinedButton.styleFrom(
                            foregroundColor: colors.error,
                          ),
                          icon: const Icon(Icons.delete_outline),
                          label: const Text('Delete'),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: FilledButton(
                          key: const ValueKey('review-save'),
                          onPressed: idle ? _save : null,
                          child: const Text('Save'),
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// A shot's stored photo, pinch to zoom, so a value can be checked against its tile: the review's, and
/// since P25 the shot's detail screen's too. Keyed `<keyPrefix>-photo`, and `<keyPrefix>-photo-shown`
/// once a frame of it is drawn. [M38 P22]
///
/// **Decoded to fit a box twice the screen's width in pixels on each side**, through [ResizeImage]'s
/// `fit`, rather than `cacheWidth` alone, which bounds only the width: a portrait photo bounded by its
/// width would still decode at nearly full height. Nothing here is a spinner, since a widget test's image
/// never finishes loading and an endless animation would keep `pumpAndSettle` from settling.
class ShotPhoto extends StatelessWidget {
  const ShotPhoto({
    super.key,
    required this.path,
    required this.missing,
    this.keyPrefix = 'review',
  });

  /// The photo's absolute path, or none, and then [missing] says why.
  final String? path;
  final String missing;
  final String keyPrefix;

  static const height = 260.0;

  /// The decode bound, as a multiple of the screen's width in pixels: sharp to twice the screen's size,
  /// which reads a tile, without holding a 12-megapixel photo whole.
  static const decodeScale = 2.0;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final path = this.path;
    final Widget shown;
    if (path == null) {
      shown = _PhotoNote(missing);
    } else {
      final media = MediaQuery.of(context);
      final bound = (media.size.width * media.devicePixelRatio * decodeScale)
          .round();
      shown = InteractiveViewer(
        key: ValueKey('$keyPrefix-photo'),
        maxScale: 8,
        child: Image(
          image: ResizeImage(
            FileImage(File(path)),
            width: bound,
            height: bound,
            policy: ResizeImagePolicy.fit,
          ),
          fit: BoxFit.contain,
          frameBuilder: (context, child, frame, _) => frame == null
              ? const _PhotoNote('Opening the photo.')
              : KeyedSubtree(
                  key: ValueKey('$keyPrefix-photo-shown'),
                  child: child,
                ),
          errorBuilder: (context, error, _) =>
              const _PhotoNote('The photo could not be shown here.'),
        ),
      );
    }
    return Container(
      height: height,
      color: theme.colorScheme.surfaceContainerHighest,
      child: ClipRect(child: shown),
    );
  }
}

/// A refusal or a reason, in Rust's words, in the error colour: the review's and the detail screen's.
class ProblemNote extends StatelessWidget {
  const ProblemNote({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: colors.errorContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Text(text, style: TextStyle(color: colors.onErrorContainer)),
    );
  }
}

class _PhotoNote extends StatelessWidget {
  const _PhotoNote(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(16),
      child: Text(text, textAlign: TextAlign.center),
    ),
  );
}

/// One tile, editable: the number as the screen prints it, its lit letter where the tile has two, and a
/// clear. Keyed `edit-<field>`, with the number `field-<field>`, the letters `sign-<field>` and the clear
/// `clear-<field>`.
///
/// A tile nothing was read for is highlighted, as on the card, and says so in its empty field. Its
/// letters start with neither chosen, since the read lit none that it saw.
class _EditRow extends StatelessWidget {
  _EditRow({
    required this.row,
    required this.field,
    required this.letter,
    required this.enabled,
    required this.onLetter,
    required this.onClear,
  }) : super(key: ValueKey('edit-${row.field}'));

  final CardRow row;
  final TextEditingController field;
  final String? letter;
  final bool enabled;
  final ValueChanged<String?> onLetter;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final missing = row.state == RowState.notRead;
    final foreground = missing ? colors.onErrorContainer : null;
    final letters = row.signLetters;
    return Container(
      color: missing ? colors.errorContainer : null,
      padding: const EdgeInsets.fromLTRB(16, 8, 4, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  row.label,
                  style: theme.textTheme.bodyLarge?.copyWith(color: foreground),
                ),
              ),
              if (row.state == RowState.corrected)
                Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: Text(
                    'corrected',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: colors.tertiary,
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 4),
          Row(
            children: [
              Expanded(
                child: TextField(
                  key: ValueKey('field-${row.field}'),
                  controller: field,
                  enabled: enabled,
                  // A minus is on the keyboard because some tiles print one. On a tile signed by a lit
                  // letter Rust refuses it and says to choose the letter, rather than this hiding a key.
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                    signed: true,
                  ),
                  textInputAction: TextInputAction.done,
                  style: theme.textTheme.titleMedium,
                  decoration: InputDecoration(
                    isDense: true,
                    filled: true,
                    fillColor: colors.surface,
                    border: const OutlineInputBorder(),
                    hintText: missing
                        ? CardRowView.notRead
                        : CardRowView.cleared,
                  ),
                ),
              ),
              if (letters != null) ...[
                const SizedBox(width: 8),
                SegmentedButton<String>(
                  key: ValueKey('sign-${row.field}'),
                  segments: [
                    for (final each in [letters.$1, letters.$2])
                      ButtonSegment(value: each, label: Text(each)),
                  ],
                  selected: {?letter},
                  emptySelectionAllowed: true,
                  showSelectedIcon: false,
                  onSelectionChanged: enabled
                      ? (picked) => onLetter(picked.firstOrNull)
                      : null,
                ),
              ],
              IconButton(
                key: ValueKey('clear-${row.field}'),
                tooltip: 'Clear ${row.label}',
                onPressed: enabled ? onClear : null,
                color: foreground,
                icon: const Icon(Icons.backspace_outlined),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// What this one shot was practised as, which can differ from the session's (C15). Keyed
/// `intent-<mode>` and `intent-shotShaping-<shape>`. The review's draft, and since P25 the detail
/// screen's action, so the two offer the same six pairs in the same words.
class IntentPicker extends StatelessWidget {
  const IntentPicker({
    super.key,
    required this.shown,
    required this.enabled,
    required this.onPick,
  });

  final (PracticeMode, TargetShape?)? shown;
  final bool enabled;
  final ValueChanged<(PracticeMode, TargetShape?)> onPick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Practised as', style: theme.textTheme.titleSmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final intent in shotIntents)
                Pick(
                  key: ValueKey(
                    [
                      'intent',
                      intent.$1.name,
                      if (intent.$2 case final shape?) shape.name,
                    ].join('-'),
                  ),
                  label: intentWord(intent.$1, intent.$2),
                  selected: intent == shown,
                  onPressed: enabled ? () => onPick(intent) : null,
                ),
            ],
          ),
        ],
      ),
    );
  }
}

/// One choice, as the change sheet draws one: a button and not a chip, for the 56 a tap needs.
class Pick extends StatelessWidget {
  const Pick({
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
