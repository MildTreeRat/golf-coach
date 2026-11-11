import 'package:flutter/material.dart';

import '../core/core_api.dart';

/// One graded topic, for a club or the player: its name, its grade, and on a shape topic both shares.
/// [M38 P26]
///
/// **The three states are Rust's, read off one field each** (ADR-034 §5.5): a graded share has a
/// `percent`, set large beside its sentence; a withheld one has none, and shows Rust's sentence alone,
/// which says what it waits for; and an absent topic is never handed to this at all. So `percent ==
/// null` is what says withheld here, and not the absence of a `%` in the text, since consistency's
/// criterion holds a `10%` of its own (P11 finding 11).
///
/// **A shape topic shows both shares** (ADR-034 §5.3), the on-line one first since it is the one a
/// blend takes, and the hit-the-shape one beneath, so a golfer can tell the shaping failing from the
/// aiming. Each share's sentence says what it is a share of, so no label of Dart's tells them apart.
/// **A projected landing is marked**: where any landing behind the on-line share came from the flight
/// model and was not printed, Rust's `landings` sentence says `projected`, and it is shown with a mark
/// beside it, keyed `projected`, so the share does not read as one the screen printed.
///
/// Keyed `topic-<name>`. Inside it: `grade-percent` and `grade-text`, then on a shape topic `hit-percent`,
/// `hit-text`, `landings` and each `no-landing`, then each `unscored`.
class TopicTile extends StatelessWidget {
  TopicTile({required this.topic})
    : super(key: ValueKey('topic-${topic.name}'));

  final TopicView topic;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final shape = topic.shape;
    final landings = shape?.landings;
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(topic.name, style: theme.textTheme.titleMedium),
            const SizedBox(height: 4),
            ShareLine(share: topic.grade, keyPrefix: 'grade'),
            if (shape != null) ...[
              const SizedBox(height: 8),
              ShareLine(share: shape.hitShape, keyPrefix: 'hit'),
              if (landings != null)
                _Landings(text: landings, projected: shape.onLineProjected),
              for (final sentence in shape.noLanding)
                _Note(key: const ValueKey('no-landing'), text: sentence),
            ],
            for (final sentence in topic.unscored)
              _Note(key: const ValueKey('unscored'), text: sentence),
          ],
        ),
      ),
    );
  }
}

/// One share: its percentage set large when it is graded, beside Rust's sentence for it. A withheld
/// share has no number to set, and its sentence stands alone. Keyed `<keyPrefix>-percent` and
/// `<keyPrefix>-text`.
class ShareLine extends StatelessWidget {
  const ShareLine({super.key, required this.share, required this.keyPrefix});

  final ShareView share;
  final String keyPrefix;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final percent = share.percent;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (percent != null) ...[
          SizedBox(
            width: 72,
            child: Text(
              percent,
              key: ValueKey('$keyPrefix-percent'),
              style: theme.textTheme.headlineSmall,
            ),
          ),
          const SizedBox(width: 8),
        ],
        Expanded(
          child: Text(
            share.text,
            key: ValueKey('$keyPrefix-text'),
            style: percent == null
                ? theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  )
                : theme.textTheme.bodyMedium,
          ),
        ),
      ],
    );
  }
}

/// Where the on-line share's landings came from, marked when any was projected rather than printed.
class _Landings extends StatelessWidget {
  const _Landings({required this.text, required this.projected});

  final String text;
  final bool projected;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final shown = Text(
      text,
      key: const ValueKey('landings'),
      style: theme.textTheme.bodySmall?.copyWith(
        color: projected ? colors.onTertiaryContainer : null,
      ),
    );
    if (!projected) {
      return Padding(padding: const EdgeInsets.only(top: 6), child: shown);
    }
    return Container(
      key: const ValueKey('projected'),
      margin: const EdgeInsets.only(top: 6),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: colors.tertiaryContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.auto_graph, size: 16, color: colors.onTertiaryContainer),
          const SizedBox(width: 6),
          Expanded(child: shown),
        ],
      ),
    );
  }
}

/// A sentence beneath a share: a shot not judged, or one with no landing. Not a miss, so not in the
/// error colour.
class _Note extends StatelessWidget {
  const _Note({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Text(
        text,
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
