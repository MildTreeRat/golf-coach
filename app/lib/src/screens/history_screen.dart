import 'dart:io';

import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import '../widgets/change_sheet.dart';
import '../widgets/shot_card.dart';

/// Opens one shot's detail from its History row, and completes once the golfer is back on the list.
/// P25's screen. [M38 P24]
typedef OpenShot = Future<void> Function(BuildContext context, ShotRow row);

/// The History tab: every shot of the current golfer's, by session, including what never counts.
/// [M38 P24]
///
/// **Everything on a row is Rust's** (rule 8): `list_sessions` hands over the order, the clock, each
/// value as a card shows it and its tile's own label, the marks, and why a shot is left out. So this
/// sorts nothing, parses no time and formats no number. A value nothing was read for has no display,
/// and shows as not read and never as 0 (ADR-010 §2).
///
/// **It reads the stores again on entering the tab**, and while it is the tab shown, after every shot
/// the Record tab's flow writes ([AppModel.shotsChanged]), since a library batch runs on while the
/// golfer looks here. A list of 500 entries took 32 to 35 ms on this Mac (P9 finding 3), so nothing is
/// cached between entries. While a read is out the last list stays up, so a refresh does not flicker.
///
/// **Marks**: `Needs review` (the stored `needs_review`), `Corrected`, `Mishit` (held out of the distance
/// averages, whoever said so) and `Drill · not counted`. A row Rust leaves out for any other reason, with
/// no mark to say so (an analysis missing, stale or outdated, a duplicate, no golfer), is marked `Not
/// counted` with Rust's sentences beneath it, since that is a shot the golfer can repair and nothing
/// else on the list would tell them. A drill's sentence is not repeated on every drill row: the mark
/// says it, and the detail screen holds the sentence.
class HistoryScreen extends StatefulWidget {
  const HistoryScreen({
    super.key,
    required this.model,
    required this.active,
    this.openShot,
  });

  final AppModel model;

  /// Whether this is the tab shown. Becoming it is what reads the stores again.
  final bool active;

  /// What a row's tap does: in the app, P25's detail screen (`showShotDetail`). None is a row that does
  /// nothing when tapped.
  final OpenShot? openShot;

  /// Dart's own sentences, under rule 8: none is about a value.
  static const noGolfer = 'No golfer is chosen, so there are no shots to list.';

  static const reading = 'Reading the shots.';

  static String noShots(GolferView golfer) =>
      'No shots for ${golfer.displayName} yet. Photograph the screen on the '
      'Record tab after each shot, and every shot is listed here, whether or '
      'not it counts.';

  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen> {
  /// The last list Rust answered, or none before the first.
  List<SessionView>? _sessions;

  /// The golfer [_sessions] are for, so a list read for one golfer is never shown under another's name.
  String? _shownFor;

  /// Rust's refusal on the last read, shown above the last list.
  String? _problem;

  /// What the last read asked about, so the model's notifications start a read only when one of them
  /// has moved, and not on every change of a setting.
  int? _askedVersion;
  String? _askedFor;

  /// Each read's number. Only the latest's answer is shown, since two can be out at once, and Rust's
  /// lock decides which finishes first.
  int _ticket = 0;

  /// The sessions the golfer opened or closed. One left alone is open if it is the newest, so the
  /// session being recorded is open on entry and an older one does not take the screen.
  final Map<String, bool> _toggled = {};

  @override
  void initState() {
    super.initState();
    widget.model.addListener(_follow);
    if (widget.active) {
      _read();
    }
  }

  @override
  void didUpdateWidget(HistoryScreen old) {
    super.didUpdateWidget(old);
    if (old.model != widget.model) {
      old.model.removeListener(_follow);
      widget.model.addListener(_follow);
    }
    if (widget.active && !old.active) {
      _read();
    }
  }

  @override
  void dispose() {
    widget.model.removeListener(_follow);
    super.dispose();
  }

  /// A shot written, or another golfer chosen, while this tab is shown. When it is not, entering it
  /// reads the stores anyway.
  ///
  /// No redraw of its own: the app's root rebuilds the Shell on every change of the model (`_Home` in
  /// `main.dart`), and that is what puts a new golfer's name in the title and takes the last golfer's
  /// list down.
  void _follow() {
    final model = widget.model;
    if (widget.active &&
        (model.shotsVersion != _askedVersion ||
            model.golfer?.playerId != _askedFor)) {
      _read();
    }
  }

  Future<void> _read() async {
    final model = widget.model;
    final ticket = ++_ticket;
    final golfer = model.golfer?.playerId;
    _askedVersion = model.shotsVersion;
    _askedFor = golfer;
    if (golfer == null) {
      // Nothing to read, and every caller draws next: no setState, since `initState` is one of them.
      _sessions = const [];
      _shownFor = null;
      _problem = null;
      return;
    }
    try {
      final sessions = await model.core.listSessions(
        root: model.root,
        playerId: golfer,
      );
      if (mounted && ticket == _ticket) {
        setState(() {
          _sessions = sessions;
          _shownFor = golfer;
          _problem = null;
        });
      }
    } on Refusal catch (refusal) {
      model.logProblem('history', refusal.message);
      if (mounted && ticket == _ticket) {
        setState(() => _problem = refusal.message);
      }
    }
  }

  bool _isOpen(SessionView session, int index) =>
      _toggled[session.sessionId] ?? index == 0;

  Future<void> _open(ShotRow row) async {
    final open = widget.openShot;
    if (open == null) {
      return;
    }
    await open(context, row);
    // The detail screen can change or delete the shot, so the list is read again on the way back.
    if (mounted) {
      _read();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final golfer = widget.model.golfer;
    final problem = _problem;
    final sessions = _shownFor == golfer?.playerId ? _sessions : null;
    final items = <Widget>[
      if (problem != null)
        _Notice(
          key: const ValueKey('history-problem'),
          text: problem,
          color: theme.colorScheme.errorContainer,
          onColor: theme.colorScheme.onErrorContainer,
        ),
      if (golfer == null)
        const _Empty(
          key: ValueKey('history-empty'),
          text: HistoryScreen.noGolfer,
        )
      else if (sessions == null)
        // No spinner: an endless animation would keep a widget test from settling, and a read is quick.
        const _Empty(text: HistoryScreen.reading)
      else if (sessions.isEmpty)
        _Empty(
          key: const ValueKey('history-empty'),
          text: HistoryScreen.noShots(golfer),
        ),
      if (sessions != null)
        for (final (index, session) in sessions.indexed) ...[
          _SessionHeader(
            session: session,
            open: _isOpen(session, index),
            onTap: () => setState(
              () => _toggled[session.sessionId] = !_isOpen(session, index),
            ),
          ),
          if (_isOpen(session, index))
            for (final row in session.rows)
              HistoryRow(
                row: row,
                onTap: widget.openShot == null ? null : () => _open(row),
              ),
        ],
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
          child: Text(
            golfer == null ? 'History' : "${golfer.displayName}'s shots",
            key: const ValueKey('history-title'),
            style: theme.textTheme.titleMedium,
          ),
        ),
        const Divider(height: 1),
        Expanded(
          // Built as it scrolls, so a session of hundreds draws only the rows on screen, and their
          // thumbnails decode only then.
          child: ListView.builder(
            key: const ValueKey('history-list'),
            itemCount: items.length,
            itemBuilder: (context, index) => items[index],
          ),
        ),
      ],
    );
  }
}

/// A session's date and its count, and a tap to open or close its rows.
class _SessionHeader extends StatelessWidget {
  _SessionHeader({required this.session, required this.open, this.onTap})
    : super(key: ValueKey('session-${session.sessionId}'));

  final SessionView session;
  final bool open;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final count = session.count;
    return Material(
      color: theme.colorScheme.surfaceContainer,
      child: ListTile(
        onTap: onTap,
        title: Text(session.date, style: theme.textTheme.titleMedium),
        subtitle: Text(count == 1 ? '1 shot' : '$count shots'),
        trailing: Icon(open ? Icons.expand_less : Icons.expand_more),
      ),
    );
  }
}

/// One shot as a line of History: its photo, which shot and when, the club and intent, carry and ball
/// speed, and its marks. Keyed `shot-<session>-<swing>`, each mark `mark-<name>`.
class HistoryRow extends StatelessWidget {
  HistoryRow({required this.row, this.onTap})
    : super(key: ValueKey('shot-${row.sessionId}-${row.swingId}'));

  final ShotRow row;
  final VoidCallback? onTap;

  /// What each mark says, by its key.
  static const flagged = 'Needs review';
  static const corrected = 'Corrected';
  static const mishit = 'Mishit';
  static const drill = 'Drill · not counted';
  static const leftOut = 'Not counted';

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final explained = !row.untracked && row.leftOut.isNotEmpty;
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Thumbnail(path: row.photoPath),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Shot ${row.swingId} · ${row.timeDisplay} · '
                    '${row.club ?? 'No club'}',
                    style: theme.textTheme.titleSmall,
                  ),
                  Text(
                    intentWord(row.mode, row.targetShape),
                    style: theme.textTheme.bodySmall,
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      Expanded(
                        child: _Value(
                          key: const ValueKey('carry'),
                          label: row.carryLabel,
                          display: row.carryDisplay,
                        ),
                      ),
                      Expanded(
                        child: _Value(
                          key: const ValueKey('ball-speed'),
                          label: row.ballSpeedLabel,
                          display: row.ballSpeedDisplay,
                        ),
                      ),
                    ],
                  ),
                  if (row.flagged ||
                      row.corrected ||
                      row.mishit ||
                      row.untracked ||
                      explained) ...[
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: [
                        if (row.flagged)
                          _Mark(
                            key: const ValueKey('mark-flagged'),
                            icon: Icons.flag,
                            text: flagged,
                            color: colors.errorContainer,
                            onColor: colors.onErrorContainer,
                          ),
                        if (row.corrected)
                          _Mark(
                            key: const ValueKey('mark-corrected'),
                            icon: Icons.edit,
                            text: corrected,
                            color: colors.tertiaryContainer,
                            onColor: colors.onTertiaryContainer,
                          ),
                        if (row.mishit)
                          _Mark(
                            key: const ValueKey('mark-mishit'),
                            icon: Icons.remove_circle_outline,
                            text: mishit,
                            color: colors.secondaryContainer,
                            onColor: colors.onSecondaryContainer,
                          ),
                        if (row.untracked)
                          _Mark(
                            key: const ValueKey('mark-drill'),
                            icon: Icons.fitness_center,
                            text: drill,
                            color: colors.surfaceContainerHighest,
                            onColor: colors.onSurface,
                          ),
                        if (explained)
                          _Mark(
                            key: const ValueKey('mark-left-out'),
                            icon: Icons.block,
                            text: leftOut,
                            color: colors.surfaceContainerHighest,
                            onColor: colors.onSurface,
                          ),
                      ],
                    ),
                  ],
                  if (explained)
                    for (final sentence in row.leftOut)
                      Padding(
                        padding: const EdgeInsets.only(top: 6),
                        child: Text(
                          sentence,
                          key: const ValueKey('left-out'),
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A tile's own label over its value as a card shows it, or over `not read` where it has none.
class _Value extends StatelessWidget {
  const _Value({super.key, required this.label, required this.display});

  final String label;
  final String? display;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final display = this.display;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: theme.textTheme.labelSmall),
        Text(
          // A row says "not read" for a cleared value too, since only the card's row can tell the two
          // apart (`RowState`), and a row carries no state per value.
          display ?? CardRowView.notRead,
          style: display == null
              ? theme.textTheme.titleMedium?.copyWith(
                  fontStyle: FontStyle.italic,
                  color: theme.colorScheme.error,
                )
              : theme.textTheme.titleMedium,
        ),
      ],
    );
  }
}

class _Mark extends StatelessWidget {
  const _Mark({
    super.key,
    required this.icon,
    required this.text,
    required this.color,
    required this.onColor,
  });

  final IconData icon;
  final String text;
  final Color color;
  final Color onColor;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
    decoration: BoxDecoration(
      color: color,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 14, color: onColor),
        const SizedBox(width: 4),
        Text(
          text,
          style: Theme.of(context).textTheme.labelMedium
              ?.copyWith(color: onColor),
        ),
      ],
    ),
  );
}

/// A shot's photo, small, beside its row. Keyed `thumbnail-photo` when a photo is named and
/// `thumbnail-none` when none is.
///
/// **Decoded to fit a box twice the thumbnail's side in pixels**, through [ResizeImage]'s `fit`, so a
/// 12-megapixel photo is never decoded whole for a 56-point square. Twice, because the square is then
/// filled by `cover`, which crops the long side: a 4:3 photo fitted into twice the box has its short side
/// at one and a half times the box, so the crop still has more pixels than it shows. The photo can be a
/// HEIC (P9 finding 10), which only the iPhone will show drawn (P30). A photo that cannot be drawn shows
/// an icon, and nothing here spins, since a widget test's image never finishes loading.
class Thumbnail extends StatelessWidget {
  const Thumbnail({super.key, required this.path});

  final String? path;

  static const side = 56.0;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final path = this.path;
    Widget icon(IconData data) =>
        Center(child: Icon(data, color: colors.onSurfaceVariant));
    final Widget shown;
    if (path == null) {
      shown = KeyedSubtree(
        key: const ValueKey('thumbnail-none'),
        child: icon(Icons.image_not_supported_outlined),
      );
    } else {
      final bound = (side * MediaQuery.devicePixelRatioOf(context) * 2).round();
      shown = Image(
        key: const ValueKey('thumbnail-photo'),
        image: ResizeImage(
          FileImage(File(path)),
          width: bound,
          height: bound,
          policy: ResizeImagePolicy.fit,
        ),
        fit: BoxFit.cover,
        frameBuilder: (context, child, frame, _) =>
            frame == null ? icon(Icons.image_outlined) : child,
        errorBuilder: (context, error, _) => icon(Icons.broken_image_outlined),
      );
    }
    return ClipRRect(
      borderRadius: BorderRadius.circular(8),
      child: Container(
        width: side,
        height: side,
        color: colors.surfaceContainerHighest,
        child: shown,
      ),
    );
  }
}

class _Empty extends StatelessWidget {
  const _Empty({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(24, 32, 24, 0),
    child: Text(
      text,
      textAlign: TextAlign.center,
      style: Theme.of(context).textTheme.bodyLarge,
    ),
  );
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
    margin: const EdgeInsets.fromLTRB(16, 12, 16, 0),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: color,
      borderRadius: BorderRadius.circular(12),
    ),
    child: Text(text, style: TextStyle(color: onColor)),
  );
}
