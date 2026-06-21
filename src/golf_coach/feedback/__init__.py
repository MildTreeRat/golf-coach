"""Feedback module — rule-based tips, LLM coaching, video overlays. [M5/M6]

Turns a `SwingResult` into a `FeedbackPayload`. Rule-based tips (M5) need no network;
the Claude coach (M6) uses the Anthropic API. The only place `anthropic` is imported.
"""
