import 'package:flutter/material.dart';

import '../core/core_api.dart';

/// What is established about a club or the player: the strengths, then the weaknesses, the largest
/// miss first, each in Rust's order and words (ADR-034 §5.7). [M38 P26]
///
/// **"Nothing established" is never a strength.** An empty list is Rust's `nothing_listed` sentence,
/// shown as it stands, and a side with nothing on it is simply not shown, so no heading here stands
/// over an empty side and nothing reads as a clean bill. The grades do not feed this list and it does
/// not feed them, which is why it sits beside them on the screen and not inside a topic.
///
/// Keyed `finding-list`. Inside it: `nothing-listed`, or `strengths` and `weaknesses`, each entry
/// `strength-<metric>` or `weakness-<metric>`. A metric is the contract's name, `carry_distance_yds`, as
/// the report prints it, since rule 8 leaves a friendlier word to Rust (P11 finding 11).
class FindingList extends StatelessWidget {
  const FindingList({
    super.key = const ValueKey('finding-list'),
    required this.strengths,
    required this.weaknesses,
    this.nothingListed,
  });

  final List<FindingView> strengths;
  final List<FindingView> weaknesses;
  final String? nothingListed;

  /// The two sides' headings, Dart's and not about a value.
  static const strengthsTitle = 'Strengths';
  static const weaknessesTitle = 'Weaknesses';

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final nothing = nothingListed;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (nothing != null)
          Text(
            nothing,
            key: const ValueKey('nothing-listed'),
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        if (strengths.isNotEmpty)
          _Side(
            key: const ValueKey('strengths'),
            title: strengthsTitle,
            prefix: 'strength',
            icon: Icons.check_circle_outline,
            findings: strengths,
          ),
        if (weaknesses.isNotEmpty)
          _Side(
            key: const ValueKey('weaknesses'),
            title: weaknessesTitle,
            prefix: 'weakness',
            icon: Icons.error_outline,
            findings: weaknesses,
          ),
      ],
    );
  }
}

class _Side extends StatelessWidget {
  const _Side({
    super.key,
    required this.title,
    required this.prefix,
    required this.icon,
    required this.findings,
  });

  final String title;
  final String prefix;
  final IconData icon;
  final List<FindingView> findings;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(title, style: theme.textTheme.labelLarge),
          for (final finding in findings)
            Padding(
              key: ValueKey('$prefix-${finding.metric}'),
              padding: const EdgeInsets.only(top: 6),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(icon, size: 18),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(finding.metric, style: theme.textTheme.titleSmall),
                        Text(finding.text, style: theme.textTheme.bodyMedium),
                      ],
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
