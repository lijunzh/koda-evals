Logs pulled from three hosts on 2026-11-04, covering 2026-10-30 to 2026-11-04.

- `scheduler.log`: sched-1, the job scheduler. It starts `nightly-export`
  at 02:00 every night, and `rollup-hourly` and `sync-inventory` during the day.
- `export-worker.log`: exp-worker-2, which runs `nightly-export` and
  `rollup-hourly`. `nightly-export` copies the previous day's orders from
  the `shop` database to S3.
- `postgres.log`: db-1, the `shop` database.
