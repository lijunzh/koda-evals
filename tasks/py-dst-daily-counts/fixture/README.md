# eventlog

Daily event counts from our service logs, in a chosen time zone.

```
python3 summarize.py --tz America/New_York logs/*.log
python3 -m unittest
```

Log lines are `<UTC timestamp> <LEVEL> <message>`, one event per line.
Standard library only (Python 3.9+).
