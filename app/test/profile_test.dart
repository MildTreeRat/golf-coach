import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/screens/profile_screen.dart';
import 'package:golf_app/src/widgets/blend_card.dart';
import 'package:golf_app/src/widgets/finding_list.dart';
import 'package:golf_app/src/widgets/topic_tile.dart';

import 'fakes.dart';
import 'history_test.dart' show enterHistory, pumpApp;

// The Profile tab over `ProfileView` fixtures, with no Rust and no Swift (C22). [M38 P26]
//
// Every sentence, percentage and mean here is one the test gave the fake, in the words `graded_profile`
// answered for the hand cases (`crates/app/tests/profile.rs` holds those to the reports), so a screen
// showing it is showing the facade's words. What these pin is what Dart does with them: a share's number
// only where Rust gave one, both shares on a shape topic, a projected landing marked, no blend card for
// no blend, an empty list in Rust's sentence and never a strength, and the selector built from the
// profile and the bag with no golf of its own.

const preamble =
    'Each grade is the share of tracked shots that met its topic, with a 95% '
    'interval, and is withheld below 10 judged shots. Launch-monitor numbers '
    'only: mechanics is not graded here.';

const nothingListed = 'nothing is established either way yet';

/// Consistency's criterion holds a `10%` of its own, so its withheld sentence does too (P11 finding 11).
const consistencyWithheld =
    'withheld, within 10% of its club\'s typical carry and line: 6 of the 10 '
    'shots a share needs';

const consistencyGraded = ShareView(
  percent: '58%',
  text:
      '58% within 10% of its club\'s typical carry and line (7 of 12), 95% CI '
      '32% .. 81%',
);

const drawWithheld =
    'withheld, hit the draw and finished within 9 yd of the line: 3 of the 10 '
    'shots a share needs';

const drawHit = ShareView(
  percent: '75%',
  text:
      '75% hit the draw, face to path beyond -2 deg (9 of 12), 95% CI 47% .. '
      '91%',
);

const projectedLandings =
    'landings: 0 printed, 3 projected by the flight model and not printed';

const printedLandings = 'landings: 3 printed';

const noLanding =
    '3 shots with no landing, so out of the on-line share: no spin flies the '
    'ball the distance the launch monitor printed';

const notJudged =
    '1 shot not judged: the launch monitor printed nothing for this number on '
    'this shot';

const carryCenter = '149.6   95% CI 148.4 .. 150.8   (median 150.0)';

const carrySpread = 'sd 2.0 yards, 95% CI 1.4 .. 3.3, n = 13';

const spreadWithheld =
    'withheld: a claim about how repeatable this is needs 10 samples (there are '
    '6)';

TopicView consistency({ShareView? grade}) => TopicView(
  name: 'consistency',
  criterion: 'within 10% of its club\'s typical carry and line',
  grade: grade ?? const ShareView(text: consistencyWithheld),
  unscored: const [],
);

TopicView draw({
  ShareView grade = const ShareView(text: drawWithheld),
  bool projected = true,
}) => TopicView(
  name: 'draw',
  criterion: 'hit the draw and finished within 9 yd of the line',
  grade: grade,
  unscored: const [notJudged],
  shape: ShapeView(
    hitShape: drawHit,
    onLineProjected: projected,
    landings: projected ? projectedLandings : printedLandings,
    noLanding: const [noLanding],
  ),
);

const blendWithDrawLeftOut = BlendView(
  mean: '100%',
  averaged: ['consistency'],
  excluded: ['draw'],
  text:
      '100%, the equal-weight mean of consistency; left out while withheld: '
      'draw',
);

ClubView clubView(
  String club, {
  int shots = 13,
  List<NumberRow>? numbers,
  List<TopicView>? topics,
  String? noTopics,
  BlendView? blend,
  List<FindingView> strengths = const [],
  List<FindingView> weaknesses = const [],
  String? nothing = nothingListed,
}) => ClubView(
  club: club,
  nShots: shots,
  shots: '$shots tracked shot photos',
  numbers:
      numbers ??
      const [
        NumberRow(
          name: 'carry_distance_yds',
          n: 13,
          center: carryCenter,
          spread: carrySpread,
          withheld: false,
        ),
      ],
  topics: topics ?? [consistency()],
  noTopics: noTopics,
  blend: blend,
  strengths: strengths,
  weaknesses: weaknesses,
  nothingListed: nothing,
);

PlayerView playerView({
  int shots = 13,
  List<TopicView>? topics,
  BlendView? blend,
}) => PlayerView(
  basis: 'each shot judged by its own club\'s criterion',
  nShots: shots,
  shots: '$shots tracked shot photos across the bag',
  topics: topics ?? [consistency()],
  blend: blend,
  strengths: const [],
  weaknesses: const [],
  nothingListed: nothingListed,
);

ProfileView profileOf(
  List<ClubView> clubs, {
  PlayerView? player,
  bool noPlayer = false,
  String? noClubs,
  String playerId = 'aaron',
}) => ProfileView(
  playerId: playerId,
  preamble: preamble,
  noClubs: noClubs,
  clubs: clubs,
  player: noPlayer ? null : (player ?? playerView()),
);

/// A golfer whose bag holds [bag], with [profile] as `graded_profile`'s answer.
FakeCore profileCore(Object profile, {List<String> bag = const ['7i']}) {
  final core = setUpCore();
  core.bags['aaron'] = [for (final club in bag) BagSlot(club: club)];
  core.profiles['aaron'] = profile;
  return core;
}

Future<void> enterProfile(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.insights_outlined));
  await tester.pumpAndSettle();
}

Finder byKey(String key) => find.byKey(ValueKey(key));

Finder inside(String key, Finder matching) =>
    find.descendant(of: byKey(key), matching: matching);

/// The text [finder] shows: its own, or the one [Text] inside it.
String textAt(WidgetTester tester, Finder finder) => tester
    .widget<Text>(
      find.descendant(of: finder, matching: find.byType(Text), matchRoot: true),
    )
    .data!;

/// Scrolls the profile until [target] is built and on screen: the list builds only what is near the
/// screen, so a section further down is not there to find until then.
Future<void> scrollTo(WidgetTester tester, Finder target) async {
  await tester.scrollUntilVisible(
    target,
    100,
    scrollable: find.descendant(
      of: byKey('profile-list'),
      matching: find.byType(Scrollable),
    ),
  );
  await tester.pumpAndSettle();
}

/// Scrolls back up to the selector and taps [key] in it.
Future<void> pick(WidgetTester tester, String key) async {
  await tester.scrollUntilVisible(
    byKey(key),
    -100,
    scrollable: find.descendant(
      of: byKey('profile-list'),
      matching: find.byType(Scrollable),
    ),
  );
  await tester.pumpAndSettle();
  await tester.tap(byKey(key));
  await tester.pumpAndSettle();
}

/// [keys], sorted by where each one's widget sits: by row, then along it.
List<String> inReadingOrder(WidgetTester tester, List<String> keys) {
  final at = {for (final key in keys) key: tester.getTopLeft(byKey(key))};
  return [...keys]..sort((a, b) {
    final dy = at[a]!.dy.compareTo(at[b]!.dy);
    return dy != 0 ? dy : at[a]!.dx.compareTo(at[b]!.dx);
  });
}

/// A pick shown as chosen, which is a check beside its label.
bool picked(WidgetTester tester, String key) =>
    inside(key, find.byIcon(Icons.check)).evaluate().isNotEmpty;

int reads(FakeCore core) =>
    core.calls.where((call) => call.startsWith('gradedProfile')).length;

void main() {
  group('the selector', () {
    testWidgets(
      'lists each graded club in Rust\'s order, then Player, and names the bag\'s clubs with no tracked shot',
      (tester) async {
        final core = profileCore(
          profileOf([clubView('driver'), clubView('7i')]),
          bag: const ['driver', '3w', '7i', 'pw'],
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        final picks = ['pick-club-driver', 'pick-club-7i', 'pick-player'];
        for (final key in picks) {
          expect(byKey(key), findsOneWidget, reason: key);
        }
        expect(inReadingOrder(tester, picks), picks);
        expect(byKey('pick-club-3w'), findsNothing);
        expect(byKey('pick-club-pw'), findsNothing);
        expect(
          textAt(tester, byKey('profile-unhit')),
          ProfileScreen.unhit(['3w', 'pw']),
        );
        expect(picked(tester, 'pick-club-driver'), isTrue);
        expect(textAt(tester, byKey('profile-heading')), 'driver');
        expect(textAt(tester, byKey('profile-preamble')), preamble);
        expect(find.text("Aaron's profile"), findsOneWidget);
      },
    );

    testWidgets(
      'a pick shows that club\'s section, and Player the player\'s, which has no numbers',
      (tester) async {
        final core = profileCore(
          profileOf([
            clubView('driver', shots: 6),
            clubView('7i', topics: [consistency(grade: consistencyGraded)]),
          ], player: playerView(shots: 19)),
          bag: const ['driver', '7i'],
        );
        await pumpApp(tester, core);
        await enterProfile(tester);
        expect(byKey('profile-unhit'), findsNothing);

        await pick(tester, 'pick-club-7i');
        expect(textAt(tester, byKey('profile-heading')), '7i');
        expect(
          textAt(tester, byKey('profile-shots')),
          '13 tracked shot photos',
        );
        expect(
          textAt(tester, inside('topic-consistency', byKey('grade-percent'))),
          '58%',
        );
        expect(byKey('number-carry_distance_yds'), findsOneWidget);
        expect(picked(tester, 'pick-club-7i'), isTrue);
        expect(picked(tester, 'pick-club-driver'), isFalse);

        await pick(tester, 'pick-player');
        expect(textAt(tester, byKey('profile-heading')), ProfileScreen.player);
        expect(
          textAt(tester, byKey('profile-shots')),
          '19 tracked shot photos across the bag',
        );
        expect(
          textAt(tester, byKey('profile-basis')),
          'each shot judged by its own club\'s criterion',
        );
        expect(byKey('number-carry_distance_yds'), findsNothing);
        expect(picked(tester, 'pick-player'), isTrue);
      },
    );

    testWidgets(
      'with no tracked shot there is no Player, and Rust\'s note says why',
      (tester) async {
        const note =
            'nothing to grade yet: no tracked shot photo is on record for this '
            'golfer.';
        final core = profileCore(
          profileOf(const [], noPlayer: true, noClubs: note),
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        expect(textAt(tester, byKey('profile-no-clubs')), note);
        expect(byKey('pick-player'), findsNothing);
        expect(byKey('profile-heading'), findsNothing);
        expect(byKey('blend-card'), findsNothing);
        expect(
          textAt(tester, byKey('profile-unhit')),
          ProfileScreen.unhit(['7i']),
        );
      },
    );

    testWidgets(
      'shots that name no club: Rust\'s note sits above the player\'s grades, shown at once',
      (tester) async {
        const note =
            'no shot names a club, so no club is listed: only the player\'s '
            'grades follow.';
        final core = profileCore(
          profileOf(const [], noClubs: note, player: playerView(shots: 4)),
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        expect(textAt(tester, byKey('profile-no-clubs')), note);
        expect(picked(tester, 'pick-player'), isTrue);
        expect(textAt(tester, byKey('profile-heading')), ProfileScreen.player);
        expect(
          tester.getTopLeft(byKey('profile-no-clubs')).dy,
          lessThan(tester.getTopLeft(byKey('profile-heading')).dy),
        );
      },
    );
  });

  group('a topic', () {
    testWidgets(
      'a withheld grade shows Rust\'s sentence and no number, though the sentence holds a %',
      (tester) async {
        final core = profileCore(profileOf([clubView('7i')]));
        await pumpApp(tester, core);
        await enterProfile(tester);

        expect(
          inside('topic-consistency', byKey('grade-percent')),
          findsNothing,
        );
        expect(
          textAt(tester, inside('topic-consistency', byKey('grade-text'))),
          consistencyWithheld,
        );
        expect(find.text('0%'), findsNothing);
      },
    );

    testWidgets(
      'a shape topic shows both shares, the on-line one first, each in its own state',
      (tester) async {
        final core = profileCore(
          profileOf([
            clubView(
              '7i',
              topics: [
                consistency(grade: consistencyGraded),
                draw(),
                TopicView(
                  name: 'fade',
                  criterion:
                      'hit the fade and finished within 9 yd of the line',
                  grade: const ShareView(
                    percent: '50%',
                    text:
                        '50% hit the fade and finished within 9 yd of the line '
                        '(5 of 10), 95% CI 24% .. 76%',
                  ),
                  unscored: const [],
                  shape: const ShapeView(
                    hitShape: ShareView(
                      percent: '80%',
                      text:
                          '80% hit the fade, face to path beyond 2 deg (8 of '
                          '10), 95% CI 49% .. 94%',
                    ),
                    onLineProjected: false,
                    landings: printedLandings,
                    noLanding: [],
                  ),
                ),
              ],
            ),
          ]),
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        // The draw: its on-line share withheld, its hit share graded.
        expect(inside('topic-draw', byKey('grade-percent')), findsNothing);
        expect(
          textAt(tester, inside('topic-draw', byKey('grade-text'))),
          drawWithheld,
        );
        expect(
          textAt(tester, inside('topic-draw', byKey('hit-percent'))),
          '75%',
        );
        expect(
          textAt(tester, inside('topic-draw', byKey('hit-text'))),
          drawHit.text,
        );
        expect(
          textAt(tester, inside('topic-draw', byKey('no-landing'))),
          noLanding,
        );
        expect(
          textAt(tester, inside('topic-draw', byKey('unscored'))),
          notJudged,
        );

        // The fade: both graded, the on-line share above the hit share.
        await scrollTo(tester, byKey('topic-fade'));
        expect(
          textAt(tester, inside('topic-fade', byKey('grade-percent'))),
          '50%',
        );
        expect(
          textAt(tester, inside('topic-fade', byKey('hit-percent'))),
          '80%',
        );
        expect(
          tester.getTopLeft(inside('topic-fade', byKey('grade-text'))).dy,
          lessThan(
            tester.getTopLeft(inside('topic-fade', byKey('hit-text'))).dy,
          ),
        );
        // A topic with no shape has no second share.
        expect(inside('topic-consistency', byKey('hit-text')), findsNothing);
      },
    );

    testWidgets(
      'a projected landing is marked projected, and a printed one is not',
      (tester) async {
        final core = profileCore(
          profileOf([
            clubView('7i', topics: [draw()]),
            clubView('pw', topics: [draw(projected: false)]),
          ]),
          bag: const ['7i', 'pw'],
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        expect(
          textAt(tester, inside('topic-draw', byKey('landings'))),
          projectedLandings,
        );
        expect(inside('topic-draw', byKey('projected')), findsOneWidget);
        expect(
          find.descendant(
            of: inside('topic-draw', byKey('projected')),
            matching: find.textContaining('projected'),
          ),
          findsOneWidget,
        );

        await pick(tester, 'pick-club-pw');
        expect(
          textAt(tester, inside('topic-draw', byKey('landings'))),
          printedLandings,
        );
        expect(byKey('projected'), findsNothing);
      },
    );

    testWidgets('no topics: Rust\'s sentence in place of the grades', (
      tester,
    ) async {
      const said =
          'no topic exists for this club yet: none of its shots printed what a '
          'topic reads';
      final core = profileCore(
        profileOf([clubView('7i', topics: const [], noTopics: said)]),
      );
      await pumpApp(tester, core);
      await enterProfile(tester);

      await scrollTo(tester, byKey('no-topics'));
      expect(textAt(tester, byKey('no-topics')), said);
      expect(find.byType(TopicTile), findsNothing);
    });
  });

  group('the blend, the numbers and the list', () {
    testWidgets(
      'a blend names what it left out, and no blend is no card and never a 0',
      (tester) async {
        final core = profileCore(
          profileOf([
            clubView(
              '7i',
              topics: [
                consistency(grade: consistencyGraded),
                draw(),
              ],
              blend: blendWithDrawLeftOut,
            ),
            clubView('pw'),
          ], player: playerView()),
          bag: const ['7i', 'pw'],
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        expect(
          textAt(tester, inside('blend-card', byKey('blend-mean'))),
          '100%',
        );
        final text = textAt(tester, inside('blend-card', byKey('blend-text')));
        expect(text, blendWithDrawLeftOut.text);
        for (final name in blendWithDrawLeftOut.excluded) {
          expect(text, contains(name));
        }

        for (final key in ['pick-club-pw', 'pick-player']) {
          await pick(tester, key);
          expect(byKey('blend-card'), findsNothing, reason: key);
          expect(byKey('blend-mean'), findsNothing, reason: key);
          expect(find.text('0%'), findsNothing, reason: key);
          expect(find.text(BlendCard.title), findsNothing, reason: key);
        }
      },
    );

    testWidgets(
      'a mean shows where Rust gives one, and a number without one shows only its spread',
      (tester) async {
        final core = profileCore(
          profileOf([
            clubView(
              '7i',
              numbers: const [
                NumberRow(
                  name: 'ball_speed_mph',
                  n: 4,
                  spread: spreadWithheld,
                  withheld: true,
                ),
                NumberRow(
                  name: 'carry_distance_yds',
                  n: 6,
                  center: carryCenter,
                  spread: spreadWithheld,
                  withheld: true,
                ),
              ],
            ),
          ]),
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        expect(
          textAt(tester, inside('number-carry_distance_yds', byKey('center'))),
          carryCenter,
        );
        expect(
          textAt(tester, inside('number-carry_distance_yds', byKey('spread'))),
          spreadWithheld,
        );
        expect(inside('number-ball_speed_mph', byKey('center')), findsNothing);
        expect(
          textAt(tester, inside('number-ball_speed_mph', byKey('spread'))),
          spreadWithheld,
        );
        expect(
          inside('number-ball_speed_mph', find.text('ball_speed_mph')),
          findsOneWidget,
        );
      },
    );

    testWidgets(
      'an empty list is Rust\'s sentence, and a list shows only the sides it has',
      (tester) async {
        const strength = FindingView(
          metric: 'start_line_offline_yds',
          text:
              'strength: established inside 0 +/- 9, its nearer bound at 0.62 '
              'of the tolerance',
        );
        const weakness = FindingView(
          metric: 'carry_distance_yds',
          text: 'weakness, scatter: the sd is 2.46 times the tolerance of 5',
        );
        final core = profileCore(
          profileOf([
            clubView('7i'),
            clubView('pw', strengths: const [strength], nothing: null),
            clubView('9i', weaknesses: const [weakness], nothing: null),
          ]),
          bag: const ['7i', 'pw', '9i'],
        );
        await pumpApp(tester, core);
        await enterProfile(tester);

        await scrollTo(tester, byKey('finding-list'));
        expect(textAt(tester, byKey('nothing-listed')), nothingListed);
        expect(byKey('strengths'), findsNothing);
        expect(byKey('weaknesses'), findsNothing);
        expect(find.text(FindingList.strengthsTitle), findsNothing);

        await pick(tester, 'pick-club-pw');
        await scrollTo(tester, byKey('finding-list'));
        expect(byKey('nothing-listed'), findsNothing);
        expect(
          inside('strength-start_line_offline_yds', find.text(strength.text)),
          findsOneWidget,
        );
        expect(byKey('weaknesses'), findsNothing);

        await pick(tester, 'pick-club-9i');
        await scrollTo(tester, byKey('finding-list'));
        expect(
          inside('weakness-carry_distance_yds', find.text(weakness.text)),
          findsOneWidget,
        );
        expect(byKey('strengths'), findsNothing);
        expect(find.text(FindingList.strengthsTitle), findsNothing);
      },
    );
  });

  group('reading the stores', () {
    testWidgets(
      'on entering the tab, and after a shot written while it is shown, and not while another tab is',
      (tester) async {
        final core = profileCore(profileOf([clubView('7i')]));
        final model = await pumpApp(tester, core);
        expect(reads(core), 0, reason: 'Record opens first');

        await enterProfile(tester);
        expect(reads(core), 1);
        model.shotsChanged();
        await tester.pumpAndSettle();
        expect(reads(core), 2);
        await model.editState(
          (state) => state.copyWith(mode: PracticeMode.performance),
        );
        await tester.pumpAndSettle();
        expect(reads(core), 2, reason: 'a setting reads nothing');

        await enterHistory(tester);
        model.shotsChanged();
        await tester.pumpAndSettle();
        expect(reads(core), 2, reason: 'Profile is not the tab shown');

        await enterProfile(tester);
        expect(reads(core), 3);
      },
    );

    testWidgets(
      'a new answer replaces the screen, and a pick whose club is gone falls back to the first',
      (tester) async {
        final core = profileCore(
          profileOf([clubView('driver'), clubView('7i')]),
          bag: const ['driver', '7i'],
        );
        final model = await pumpApp(tester, core);
        await enterProfile(tester);
        await pick(tester, 'pick-club-7i');
        expect(textAt(tester, byKey('profile-heading')), '7i');

        // Kept across a read that still holds it.
        core.profiles['aaron'] = profileOf([
          clubView('driver'),
          clubView('7i', shots: 14),
        ]);
        model.shotsChanged();
        await tester.pumpAndSettle();
        expect(textAt(tester, byKey('profile-heading')), '7i');
        expect(
          textAt(tester, byKey('profile-shots')),
          '14 tracked shot photos',
        );

        // Its last shot retagged away: the first club stands in.
        core.profiles['aaron'] = profileOf([clubView('driver')]);
        model.shotsChanged();
        await tester.pumpAndSettle();
        expect(textAt(tester, byKey('profile-heading')), 'driver');
        expect(
          textAt(tester, byKey('profile-unhit')),
          ProfileScreen.unhit(['7i']),
        );
      },
    );

    testWidgets('another golfer chosen reads theirs, under their name', (
      tester,
    ) async {
      final core = profileCore(profileOf([clubView('7i')]));
      const bea = GolferView(
        playerId: 'bea',
        displayName: 'Bea',
        handedness: Handedness.left,
      );
      core.golfers.add(bea);
      core.bags['bea'] = const [BagSlot(club: 'driver')];
      core.profiles['bea'] = profileOf([
        clubView('driver', shots: 3),
      ], playerId: 'bea');
      final model = await pumpApp(tester, core);
      await enterProfile(tester);
      expect(textAt(tester, byKey('profile-heading')), '7i');

      await model.chooseGolfer('bea');
      await tester.pumpAndSettle();
      expect(find.text("Bea's profile"), findsOneWidget);
      expect(textAt(tester, byKey('profile-heading')), 'driver');
      expect(textAt(tester, byKey('profile-shots')), '3 tracked shot photos');
      expect(core.calls.last, 'gradedProfile bea');
    });

    testWidgets('a refusal shows in Rust\'s words', (tester) async {
      const refused =
          'the graded profile failed its own check, GradedProfile.clubs: out of '
          'bag order. That is a bug in a builder, not a fact about the golfer.';
      final core = profileCore(refused);
      await pumpApp(tester, core);
      await enterProfile(tester);

      expect(textAt(tester, byKey('profile-problem')), refused);
      expect(byKey('profile-heading'), findsNothing);
    });
  });
}
