import 'package:flutter/material.dart';

import '../core/core_api.dart';

/// A club's or the player's blend: the mean set large, and Rust's sentence beneath it, which names
/// the topics it averaged and the withheld ones it left out (ADR-034 §5.4 and §5.5). [M38 P26]
///
/// **Only ever built for a blend that exists.** A blend with no graded topic is absent, not 0, so the
/// Profile tab builds no card at all for one and never this with an empty value (rule 8). The mean has
/// no interval, since none has been derived, and nothing here adds one. Keyed `blend-card`, the mean
/// `blend-mean` and the sentence `blend-text`.
class BlendCard extends StatelessWidget {
  const BlendCard({
    super.key = const ValueKey('blend-card'),
    required this.blend,
  });

  final BlendView blend;

  /// The card's heading, Dart's and not about a value.
  static const title = 'Blend';

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 16, 16, 0),
      color: colors.secondaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              title,
              style: theme.textTheme.titleSmall?.copyWith(
                color: colors.onSecondaryContainer,
              ),
            ),
            Text(
              blend.mean,
              key: const ValueKey('blend-mean'),
              style: theme.textTheme.displaySmall?.copyWith(
                color: colors.onSecondaryContainer,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              blend.text,
              key: const ValueKey('blend-text'),
              style: theme.textTheme.bodyMedium?.copyWith(
                color: colors.onSecondaryContainer,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
