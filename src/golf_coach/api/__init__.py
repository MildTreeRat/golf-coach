"""API module — FastAPI orchestrator (the imperative shell).

Wires the modules together: capture -> pose/detection -> analysis -> feedback, pulling
shot data from the launch_monitor source. The only component that knows about all the
others; everything else stays decoupled. Requires the `api` extra.
"""
