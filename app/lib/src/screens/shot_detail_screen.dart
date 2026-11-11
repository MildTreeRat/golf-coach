import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import '../widgets/shot_card.dart';
import 'review_screen.dart';

/// Push one shot's detail over History, completing once the golfer is back on the list, which History
/// then reads again (P24 finding 5). [M38 P25]
Future<void> showShotDetail(
  BuildContext context,
  AppModel model,
  ShotRow row,
) => Navigator.of(context).push<void>(
  MaterialPageRoute(
    builder: (_) => ShotDetailScreen(model: model, row: row),
  ),
);

/// One shot, everything stored about it, and what a person can do to it (D10). [M38 P25]
///
/// **Everything shown is `shot_detail`'s** (rule 8): the photo through the manifest, the card, each
/// measurement with the line that says how it was taken, what could not be measured in the contract's
/// sentences, the notes, and why the corpus leaves the shot out. So this formats no number and words no
/// reason. A measurement's name is the contract's key (`flight_carry_yds`), since no label table exists
/// for them and one belongs in Rust if it is wanted (the plan's P9 finding 6).
///
/// **Each action is one facade call that answers the screen as it now stands** (P10 finding 10), so a
/// club, an intent or a verdict is written as it is tapped, with no draft and no Save, as the change
/// sheet's picks are, and the answer replaces what is shown. Each can be said again, which moves no
/// byte but re-runs the analysis, so a tap on the club already chosen is also the repair of a lost
/// analysis. A refusal shows as Rust's sentence and changes nothing. One action at a time: every control
/// is disabled while one is out. Delete asks first, since nothing undoes it, and then goes back to History.
///
/// **Editing values is the review, opened not forced**, even on a flagged shot. A flagged shot found here
/// is one whose forced review was cut off by the app closing (P22 finding 6). Forcing it again here
/// would hold someone who came to delete a stray photo on a screen that can only save, so the shot keeps
/// its `Needs review` mark and this screen says what releases it.
///
/// **An entry with no card offers no edit, club or intent**: `set_shot_club` and `set_shot_intent`
/// refuse an entry whose shot is gone, so a verdict and a delete are what is left (P10 finding 4), and
/// `no_card` says why.
class ShotDetailScreen extends StatefulWidget {
  const ShotDetailScreen({super.key, required this.model, required this.row});

  final AppModel model;

  /// History's row, which names the shot, and titles the screen until the detail arrives.
  final ShotRow row;

  /// Dart's own sentences, under rule 8: none is about a value.
  static const reading = 'Reading the shot.';

  static const flagged =
      'This read has not been reviewed. Edit its values, check each against '
      'the photo, then save.';

  static const noPhoto = 'No photo is stored for this shot.';

  static String notInBag(String club) =>
      'This shot is tagged $club, which is not in the bag.';

  /// What is in force, and whose call it is: `mishit` is the corpus's answer, and `mishit_verdict` a
  /// person's, with none meaning the automatic rule (P9 finding 11). Both are read and nothing is
  /// decided, so the line is true whichever way the two came to stand.
  static String mishitInForce(ShotRow row) =>
      switch ((row.mishitVerdict, row.mishit)) {
        (null, true) =>
          'The automatic rule calls this a mishit, so it is held out of the '
              'distance averages.',
        (null, false) => 'The automatic rule does not call this a mishit.',
        (_, true) => 'Your call: a mishit, held out of the distance averages.',
        (_, false) => 'Your call: a real shot, not held out as a mishit.',
      };

  @override
  State<ShotDetailScreen> createState() => _ShotDetailScreenState();
}

/// The mishit control's three positions, since a segmented button needs a value for "no verdict".
enum _Verdict {
  automatic('Automatic', null),
  confirmed('Mishit', MishitVerdict.confirmed),
  cleared('Not a mishit', MishitVerdict.cleared);

  const _Verdict(this.label, this.verdict);

  final String label;
  final MishitVerdict? verdict;

  static _Verdict of(MishitVerdict? verdict) =>
      values.firstWhere((each) => each.verdict == verdict);
}

class _ShotDetailScreenState extends State<ShotDetailScreen> {
  ShotDetail? _detail;

  /// `shot_detail`'s refusal: the entry is not on this phone, so nothing is left to show or do.
  String? _goneBecause;

  /// The last action's refusal, in Rust's words, shown until the next action.
  String? _problem;

  bool _busy = false;

  AppModel get _model => widget.model;

  /// The row as last answered, or History's until the detail arrives.
  ShotRow get _row => _detail?.row ?? widget.row;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final detail = await _model.core.shotDetail(
        root: _model.root,
        sessionId: widget.row.sessionId,
        swingId: widget.row.swingId,
      );
      if (mounted) {
        setState(() {
          _detail = detail;
          _goneBecause = null;
        });
      }
    } on Refusal catch (refusal) {
      _model.logProblem('shot detail', refusal.message);
      if (mounted) {
        setState(() => _goneBecause = refusal.message);
      }
    }
  }

  /// This screen's shot, as an event names it.
  String get _shot => shotOf(widget.row.sessionId, widget.row.swingId);

  /// One writer, whose answer is the screen as it now stands, logged as [kind] with [what] after the
  /// shot (D16) [M38 P30b]. A refusal wrote nothing, so the detail shown stands with it, and it is
  /// logged as an `error`.
  Future<void> _act(
    EventKind kind,
    String what,
    Future<ShotDetail> Function() write,
  ) async {
    setState(() {
      _busy = true;
      _problem = null;
    });
    try {
      final detail = await write();
      _model.logEvent(kind, '$_shot $what');
      if (mounted) {
        setState(() => _detail = detail);
      }
      // History reads again on the way back anyway. This is for whatever else shows the stores.
      _model.shotsChanged();
    } on Refusal catch (refusal) {
      _model.logProblem('shot detail', refusal.message);
      if (mounted) {
        setState(() => _problem = refusal.message);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  void _setClub(String club) => _act(
    EventKind.clubSet,
    club,
    () => _model.core.setShotClub(
      root: _model.root,
      sessionId: widget.row.sessionId,
      swingId: widget.row.swingId,
      club: club,
      now: _model.now(),
    ),
  );

  void _setIntent((PracticeMode, TargetShape?) intent) => _act(
    EventKind.intentSet,
    [intent.$1.name, ?intent.$2?.name].join(' '),
    () => _model.core.setShotIntent(
      root: _model.root,
      sessionId: widget.row.sessionId,
      swingId: widget.row.swingId,
      mode: intent.$1,
      targetShape: intent.$2,
      now: _model.now(),
    ),
  );

  void _setVerdict(_Verdict verdict) => _act(
    EventKind.mishitSet,
    verdict.name,
    () => _model.core.setShotMishit(
      root: _model.root,
      sessionId: widget.row.sessionId,
      swingId: widget.row.swingId,
      verdict: verdict.verdict,
      now: _model.now(),
    ),
  );

  /// The review over this screen, not forced. However it ends the stores may have moved, a back-out
  /// included, since a review can store an intent before its values are refused (P22 finding 3), so
  /// the detail is read again. A review that ends with the entry gone ends this screen too.
  Future<void> _edit(ShotCard card) async {
    final end = await showReview(context, _model, card, forced: false);
    if (!mounted) {
      return;
    }
    _model.shotsChanged();
    if (end is ReviewDeleted) {
      Navigator.of(context).pop();
      return;
    }
    await _load();
  }

  Future<void> _delete() async {
    if (!await confirmDelete(context, widget.row.swingId) || !mounted) {
      return;
    }
    setState(() {
      _busy = true;
      _problem = null;
    });
    try {
      await _model.core.deleteShot(
        root: _model.root,
        sessionId: widget.row.sessionId,
        swingId: widget.row.swingId,
      );
      _model.logEvent(EventKind.shotDeleted, _shot);
      _model.shotsChanged();
      if (mounted) {
        Navigator.of(context).pop();
      }
    } on Refusal catch (refusal) {
      _model.logProblem('delete', refusal.message);
      if (!mounted) {
        return;
      }
      setState(() => _problem = refusal.message);
      // `delete_shot` can refuse after the entry has gone (the review's reason), and looking again is
      // how this screen learns there is nothing left to act on.
      await _load();
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final row = _row;
    final detail = _detail;
    final gone = _goneBecause;
    final problem = _problem;
    final idle = !_busy;
    final card = detail?.card;
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Shot ${row.swingId}'),
            Text(
              '${row.timeDisplay} · ${row.club ?? 'No club'}',
              key: const ValueKey('detail-heading'),
              style: theme.textTheme.bodySmall,
            ),
          ],
        ),
      ),
      // Built whole rather than as it scrolls: a shot's rows are tens at most, and the review lays its
      // rows out the same way.
      body: SingleChildScrollView(
        key: const ValueKey('detail-list'),
        padding: const EdgeInsets.only(bottom: 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (gone != null)
              Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    // A delete's refusal, when looking again then found the entry gone: both are true.
                    if (problem != null)
                      ProblemNote(
                        key: const ValueKey('detail-problem'),
                        text: problem,
                      ),
                    ProblemNote(key: const ValueKey('detail-gone'), text: gone),
                  ],
                ),
              )
            else ...[
              ShotPhoto(
                keyPrefix: 'detail',
                path: row.photoPath,
                missing: detail == null
                    ? ShotDetailScreen.reading
                    : ShotDetailScreen.noPhoto,
              ),
              if (detail == null)
                const _Plain(ShotDetailScreen.reading)
              else ...[
                if (row.flagged)
                  _Flagged(key: const ValueKey('detail-flagged')),
                if (row.leftOut.isNotEmpty)
                  _Section(
                    key: const ValueKey('detail-left-out'),
                    title: 'Not counted',
                    children: [
                      for (final sentence in row.leftOut)
                        _Line(key: const ValueKey('left-out'), text: sentence),
                    ],
                  ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                  child: card != null
                      ? ShotCardView(
                          card: card,
                          onTap: idle ? () => _edit(card) : null,
                        )
                      : _Plain(
                          detail.noCard ?? '',
                          key: const ValueKey('detail-no-card'),
                        ),
                ),
                if (card != null) ...[
                  _clubPicker(row, idle),
                  IntentPicker(
                    shown: (row.mode, row.targetShape),
                    enabled: idle,
                    onPick: _setIntent,
                  ),
                ],
                _mishit(row, idle),
                if (detail.measurements.isNotEmpty)
                  _Section(
                    title: 'Measured',
                    children: [
                      for (final each in detail.measurements)
                        _Measurement(each),
                    ],
                  ),
                // A flight refusal is the usual row here, not the exception (P9 finding 4), so this list is
                // laid out as plainly as the measurements and not in the error colour.
                if (detail.unscored.isNotEmpty)
                  _Section(
                    title: 'Not measured',
                    children: [
                      for (final each in detail.unscored) _Unscored(each),
                    ],
                  ),
                if (detail.notes.isNotEmpty)
                  _Section(
                    title: 'Notes',
                    children: [
                      for (final note in detail.notes)
                        _Line(key: const ValueKey('note'), text: note),
                    ],
                  ),
              ],
            ],
          ],
        ),
      ),
      bottomNavigationBar: detail == null || gone != null
          ? null
          : SafeArea(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    // An action's refusal, in Rust's words, above the buttons and so always on screen,
                    // wherever in the list the control that met it is.
                    if (problem != null)
                      ProblemNote(
                        key: const ValueKey('detail-problem'),
                        text: problem,
                      ),
                    Row(
                      children: [
                        Expanded(
                          child: OutlinedButton.icon(
                            key: const ValueKey('detail-delete'),
                            onPressed: idle ? _delete : null,
                            style: OutlinedButton.styleFrom(
                              foregroundColor: theme.colorScheme.error,
                            ),
                            icon: const Icon(Icons.delete_outline),
                            label: const Text('Delete'),
                          ),
                        ),
                        if (card != null) ...[
                          const SizedBox(width: 12),
                          Expanded(
                            child: FilledButton.icon(
                              key: const ValueKey('detail-edit'),
                              onPressed: idle ? () => _edit(card) : null,
                              icon: const Icon(Icons.edit_outlined),
                              label: const Text('Edit values'),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
            ),
    );
  }

  /// The bag's clubs (D7), the shot's own chosen. A club the bag no longer holds is not offered, and
  /// is said, so the golfer sees why nothing is chosen.
  Widget _clubPicker(ShotRow row, bool idle) {
    final club = row.club;
    final inBag = _model.bag.any((slot) => slot.club == club);
    return _Section(
      title: 'Club',
      children: [
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final slot in _model.bag)
              Pick(
                key: ValueKey('detail-club-${slot.club}'),
                label: slot.club,
                selected: slot.club == club,
                onPressed: idle ? () => _setClub(slot.club) : null,
              ),
          ],
        ),
        if (club != null && !inBag)
          _Line(
            key: const ValueKey('detail-club-not-in-bag'),
            text: ShotDetailScreen.notInBag(club),
          ),
      ],
    );
  }

  Widget _mishit(ShotRow row, bool idle) => _Section(
    title: 'Mishit',
    children: [
      SegmentedButton<_Verdict>(
        key: const ValueKey('mishit'),
        segments: [
          for (final each in _Verdict.values)
            ButtonSegment(
              value: each,
              label: Text(each.label, key: ValueKey('mishit-${each.name}')),
            ),
        ],
        selected: {_Verdict.of(row.mishitVerdict)},
        showSelectedIcon: false,
        onSelectionChanged: idle
            ? (picked) => _setVerdict(picked.single)
            : null,
      ),
      _Line(
        key: const ValueKey('mishit-in-force'),
        text: ShotDetailScreen.mishitInForce(row),
      ),
    ],
  );
}

class _Flagged extends StatelessWidget {
  const _Flagged({super.key});

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
      child: Row(
        children: [
          Icon(Icons.flag, color: colors.error),
          const SizedBox(width: 8),
          const Expanded(child: Text(ShotDetailScreen.flagged)),
        ],
      ),
    );
  }
}

/// A heading and what is under it.
class _Section extends StatelessWidget {
  const _Section({super.key, required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 16, 16, 0),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(title, style: Theme.of(context).textTheme.titleSmall),
        const SizedBox(height: 8),
        ...children,
      ],
    ),
  );
}

/// One sentence, as given.
class _Line extends StatelessWidget {
  const _Line({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: 4),
    child: Text(text, style: Theme.of(context).textTheme.bodyMedium),
  );
}

class _Plain extends StatelessWidget {
  const _Plain(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(16),
    child: Text(text, style: Theme.of(context).textTheme.bodyLarge),
  );
}

/// One number measured from the shot: the contract's name, the value and unit as stored, and the line
/// saying how it was taken, which is what keeps a simulated carry from reading as a measured one (P9
/// finding 11). Keyed `measure-<name>`.
class _Measurement extends StatelessWidget {
  _Measurement(this.row) : super(key: ValueKey('measure-${row.name}'));

  final MeasurementRow row;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Expanded(child: Text(row.name, style: theme.textTheme.bodyLarge)),
              Text(row.display, style: theme.textTheme.titleMedium),
              const SizedBox(width: 4),
              Text(row.unit, style: theme.textTheme.bodySmall),
            ],
          ),
          Text(row.detail, style: theme.textTheme.bodySmall),
        ],
      ),
    );
  }
}

/// One thing that could not be produced: the tile's own label and the contract's sentence. Keyed
/// `unscored-<name>`.
class _Unscored extends StatelessWidget {
  _Unscored(this.row) : super(key: ValueKey('unscored-${row.name}'));

  final UnscoredRow row;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(row.label, style: theme.textTheme.bodyLarge),
          Text(row.reason, style: theme.textTheme.bodySmall),
        ],
      ),
    );
  }
}
