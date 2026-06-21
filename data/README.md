# data/

Local data — **gitignored** (videos, models, and the DB are large/binary). This README
keeps the directory in git and documents the layout.

```
data/
├── raw/         # raw swing video files, e.g. raw/sessions/{session_id}/   (gitignored)
├── processed/   # extracted keypoints, labeled images, intermediate artifacts (gitignored)
├── models/      # trained model weights, *.pt/*.onnx                        (gitignored)
└── golf_trainer.db   # SQLite database                                      (gitignored)
```

Drop a sample/phone swing clip into `data/raw/` to start Milestone 1.
