import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import '../widgets/blend_card.dart';
import '../widgets/finding_list.dart';
import '../widgets/topic_tile.dart';
import 'review_screen.dart' show Pick;

/// The Profile tab: the point of the product, each club's grades and what is established, then the
/// player's (the program plan's M38 P3). [M38 P26]
///
/// **Everything about a value is Rust's** (rule 8): `graded_profile` hands over the clubs in bag order,
/// each grade's percentage and sentence, the blends, the means, the spreads and the lists, and every
/// sentence is `golf-core profile`'s but a mean, which is `golf-core club-profile`'s. So this sorts
/// nothing, rounds nothing, and decides no state: a share with no `percent` is withheld and shows
/// Rust's sentence with no number, a topic Rust leaves out is shown nowhere, a club or player with no
/// blend has no blend card and never a 0, and an empty list is Rust's `nothing_listed` sentence and
/// never a strength (ADR-034 §5.5 and §5.7).
///
/// **The selector** lists each club with a tracked shot, in Rust's order, then Player when the golfer
/// has any tracked shot. A club in the bag with no shots is no club of the profile, so it is not in the
/// selector, and the line beneath it names those clubs: a set difference of the bag and the profile's
/// clubs, with no golf in it, since no string for it crosses (P11 finding 11). The first club is shown
/// until the golfer picks, and the pick is kept while it still exists.
///
/// **It reads the stores again on entering the tab, and while it is the tab shown, after every shot
/// written** ([AppModel.shotsChanged]) or another golfer chosen, as History does (P24 finding 6), since
/// a retag, a verdict or a delete on History's detail screen moves a grade, and a library batch runs on
/// while the golfer looks here. While a read is out the last profile stays up.
class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key, required this.model, required this.active});

  final AppModel model;

  /// Whether this is the tab shown. Becoming it is what reads the stores again.
  final bool active;

  /// Dart's own sentences and labels, under rule 8: none is about a value.
  static const noGolfer =
      'No golfer is chosen, so there is no profile to show.';

  static const reading = 'Reading the profile.';

  static const player = 'Player';

  static const numbersTitle = 'Numbers';

  static const gradesTitle = 'Grades';

  static const listTitle = 'Established';

  static String title(GolferView golfer) => "${golfer.displayName}'s profile";

  /// The bag's clubs with no tracked shot, in the bag's order. "Tracked", since a club hit only in
  /// drills is not in the profile either.
  static String unhit(List<String> clubs) =>
      'In the bag with no tracked shot, so not in the profile: '
      '${clubs.join(', ')}.';

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  /// The last profile Rust answered, or none before the first.
  ProfileView? _view;

  /// The golfer [_view] is for, so a profile read for one golfer is never shown under another's name.
  String? _shownFor;

  /// Rust's refusal on the last read, shown above the last profile.
  String? _problem;

  /// What the last read asked about, so the model's notifications start a read only when one of them
  /// has moved, and not on every change of a setting.
  int? _askedVersion;
  String? _askedFor;

  /// Each read's number. Only the latest's answer is shown, since two can be out at once.
  int _ticket = 0;

  /// What the golfer picked: a club's id, or the player. None picked is the first club.
  String? _pickedClub;
  bool _pickedPlayer = false;

  @override
  void initState() {
    super.initState();
    widget.model.addListener(_follow);
    if (widget.active) {
      _read();
    }
  }

  @override
  void didUpdateWidget(ProfileScreen old) {
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
  /// reads the stores anyway. No redraw of its own: the app's root rebuilds the Shell on every change
  /// of the model, which is what puts a new golfer's name in the title.
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
      _view = null;
      _shownFor = null;
      _problem = null;
      return;
    }
    try {
      final view = await model.core.gradedProfile(
        root: model.root,
        playerId: golfer,
      );
      if (mounted && ticket == _ticket) {
        setState(() {
          if (_shownFor != golfer) {
            _pickedClub = null;
            _pickedPlayer = false;
          }
          _view = view;
          _shownFor = golfer;
          _problem = null;
        });
      }
    } on Refusal catch (refusal) {
      model.logProblem('profile', refusal.message);
      if (mounted && ticket == _ticket) {
        setState(() => _problem = refusal.message);
      }
    }
  }

  void _pick({String? club, bool player = false}) => setState(() {
    _pickedClub = club;
    _pickedPlayer = player;
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final golfer = widget.model.golfer;
    final problem = _problem;
    final view = _shownFor == golfer?.playerId ? _view : null;
    final items = <Widget>[
      if (problem != null)
        _Notice(key: const ValueKey('profile-problem'), text: problem),
      if (golfer == null)
        const _Plain(
          key: ValueKey('profile-empty'),
          text: ProfileScreen.noGolfer,
        )
      else if (view == null)
        // No spinner: an endless animation would keep a widget test from settling, and a read is quick.
        const _Plain(text: ProfileScreen.reading)
      else
        ..._profile(context, view),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
          child: Text(
            golfer == null ? 'Profile' : ProfileScreen.title(golfer),
            key: const ValueKey('profile-title'),
            style: theme.textTheme.titleMedium,
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: ListView(
            key: const ValueKey('profile-list'),
            padding: const EdgeInsets.only(bottom: 24),
            children: items,
          ),
        ),
      ],
    );
  }

  List<Widget> _profile(BuildContext context, ProfileView view) {
    final theme = Theme.of(context);
    final noClubs = view.noClubs;
    final player = view.player;
    final graded = {for (final club in view.clubs) club.club};
    final unhit = [
      for (final slot in widget.model.bag)
        if (!graded.contains(slot.club)) slot.club,
    ];
    final picked = view.clubs
        .where((club) => club.club == _pickedClub)
        .firstOrNull;
    final showPlayer = player != null && (_pickedPlayer || view.clubs.isEmpty);
    final club = showPlayer ? null : (picked ?? view.clubs.firstOrNull);
    return [
      Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
        child: Text(
          view.preamble,
          key: const ValueKey('profile-preamble'),
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ),
      if (noClubs != null)
        _Plain(key: const ValueKey('profile-no-clubs'), text: noClubs),
      if (view.clubs.isNotEmpty || player != null)
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final each in view.clubs)
                Pick(
                  key: ValueKey('pick-club-${each.club}'),
                  label: each.club,
                  selected: each.club == club?.club,
                  onPressed: () => _pick(club: each.club),
                ),
              if (player != null)
                Pick(
                  key: const ValueKey('pick-player'),
                  label: ProfileScreen.player,
                  selected: showPlayer,
                  onPressed: () => _pick(player: true),
                ),
            ],
          ),
        ),
      if (unhit.isNotEmpty)
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
          child: Text(
            ProfileScreen.unhit(unhit),
            key: const ValueKey('profile-unhit'),
            style: theme.textTheme.bodySmall,
          ),
        ),
      if (club != null)
        ..._club(context, club)
      else if (showPlayer)
        ..._player(context, player),
    ];
  }

  List<Widget> _club(BuildContext context, ClubView club) => [
    _Heading(title: club.club, shots: club.shots),
    if (club.blend case final blend?) BlendCard(blend: blend),
    if (club.numbers.isNotEmpty)
      _Section(
        title: ProfileScreen.numbersTitle,
        children: [for (final row in club.numbers) _Number(row)],
      ),
    _grades(club.topics, club.noTopics),
    _list(club.strengths, club.weaknesses, club.nothingListed),
  ];

  List<Widget> _player(BuildContext context, PlayerView player) => [
    _Heading(
      title: ProfileScreen.player,
      shots: player.shots,
      basis: player.basis,
    ),
    if (player.blend case final blend?) BlendCard(blend: blend),
    _grades(player.topics, player.noTopics),
    _list(player.strengths, player.weaknesses, player.nothingListed),
  ];

  Widget _grades(List<TopicView> topics, String? noTopics) => _Section(
    title: ProfileScreen.gradesTitle,
    children: [
      if (noTopics != null)
        _Line(key: const ValueKey('no-topics'), text: noTopics),
      for (final topic in topics) TopicTile(topic: topic),
    ],
  );

  Widget _list(
    List<FindingView> strengths,
    List<FindingView> weaknesses,
    String? nothingListed,
  ) => _Section(
    title: ProfileScreen.listTitle,
    children: [
      FindingList(
        strengths: strengths,
        weaknesses: weaknesses,
        nothingListed: nothingListed,
      ),
    ],
  );
}

/// What is shown: a club's id or Player, with how many shots stand behind it, and for the player what
/// its grades pool. Keyed `profile-heading`, `profile-shots` and `profile-basis`.
class _Heading extends StatelessWidget {
  const _Heading({required this.title, required this.shots, this.basis});

  final String title;
  final String shots;
  final String? basis;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final basis = this.basis;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            key: const ValueKey('profile-heading'),
            style: theme.textTheme.headlineSmall,
          ),
          Text(
            shots,
            key: const ValueKey('profile-shots'),
            style: theme.textTheme.bodyMedium,
          ),
          if (basis != null)
            Text(
              basis,
              key: const ValueKey('profile-basis'),
              style: theme.textTheme.bodySmall,
            ),
        ],
      ),
    );
  }
}

/// One raw number: the contract's name, its mean set large once it clears its floor, and its spread or
/// why that is withheld. A mean not yet through shows nothing in its place, and never a 0. Keyed
/// `number-<name>`, and inside it `center` and `spread`.
class _Number extends StatelessWidget {
  _Number(this.row) : super(key: ValueKey('number-${row.name}'));

  final NumberRow row;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final center = row.center;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(row.name, style: theme.textTheme.titleSmall),
          if (center != null)
            Text(
              center,
              key: const ValueKey('center'),
              style: theme.textTheme.titleLarge,
            ),
          Text(
            row.spread,
            key: const ValueKey('spread'),
            style: theme.textTheme.bodyMedium?.copyWith(
              color: row.withheld ? theme.colorScheme.onSurfaceVariant : null,
            ),
          ),
        ],
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 20, 16, 0),
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

class _Line extends StatelessWidget {
  const _Line({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) =>
      Text(text, style: Theme.of(context).textTheme.bodyMedium);
}

class _Plain extends StatelessWidget {
  const _Plain({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 16, 16, 0),
    child: Text(text, style: Theme.of(context).textTheme.bodyLarge),
  );
}

class _Notice extends StatelessWidget {
  const _Notice({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      margin: const EdgeInsets.fromLTRB(16, 12, 16, 0),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: colors.errorContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Text(text, style: TextStyle(color: colors.onErrorContainer)),
    );
  }
}
